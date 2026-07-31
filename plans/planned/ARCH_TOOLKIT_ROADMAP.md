# arch-toolkit Unified Roadmap

**Status:** Planned  
**Last reviewed:** 2026-07-31  
**Scope:** `arch-toolkit` repository, with a final external handoff to Pacsea

This is the sole planning source of truth for the repository. New repository TODOs belong here rather than in separate phase plans or proposals. When every in-repository phase is complete and verified, move this file to `plans/archive/`.

## Goal

Ship the substantial post-v0.2.0 work as a coherent release, then improve dependency resolution and reliability before adding lower-priority integrations. Preserve backward compatibility, deterministic tests, feature isolation, dry-run safety, and graceful behavior when Arch tools are unavailable.

## Repository Overview

- Rust 2024 library crate (`arch-toolkit`), currently declared as version `0.2.0` with MSRV 1.91.
- Feature-gated modules:
  - `aur` (default): AUR search, package details, comments, PKGBUILD retrieval, retries, validation, and caching.
  - `deps`: package parsing, queries, version checks, dependency resolution, reverse-dependency analysis, and source classification.
  - `index`: installed/explicit package queries, official repository indexes, repository discovery, persistence, and search.
  - `install`: shell-safe pacman/AUR-helper command construction and mixed install planning.
  - `news`: Arch news and security-advisory feed parsing/fetching.
  - `sandbox`: PKGBUILD/.SRCINFO build-preflight dependency analysis.
  - `fuzzy-search` and `cache-disk`: optional search and persistence capabilities.
- Primary architecture:
  - `src/lib.rs` and `src/prelude.rs` define the public feature-gated surface.
  - `src/client.rs` owns AUR HTTP configuration, rate limiting, retries, validation, and cache integration.
  - Domain implementations live in `src/{aur,deps,index,install,news,sandbox}`.
  - Shared public models live in `src/types`; crate-wide errors live in `src/error.rs`.
  - Examples mirror public workflows; six integration-test targets cover cache, dependencies, index, install, news, and sandbox.
- Delivery infrastructure includes GitHub CI/release workflows and local scripts under `dev/scripts`.

## Current-State Findings

1. `v0.2.0` is already tagged at commit `83b5c34` (2025-12-24), while `main` contains roughly 9,800 added lines across 66 changed files after that tag. The current crate version is still `0.2.0`; publishing another artifact with that version is not a valid next step.
2. The post-tag work adds major public capabilities (`index`, `install`, `news`, `sandbox`) and migration helpers. The next release should therefore default to a new minor version (provisionally `0.3.0`), subject to a public-API compatibility audit.
3. `CHANGELOG.md` is stale and internally inconsistent: its v0.2.0 section omits most post-tag work and contains conflicting/placeholder dates.
4. No `TODO`, `FIXME`, `XXX`, or `HACK` markers were found in Rust source. Open work was concentrated in the superseded planning documents consolidated here.
5. Internal pacman/AUR-helper parsing commands consistently set `LC_ALL=C` and `LANG=C`. Configurable localized labels are only justified for callers that pass externally captured localized output into the public parsers; they are not needed by current internal command paths.
6. The historical migration guide describes changes in another repository. Pacsea cutover remains an external workstream and must not be reported as completed by changes in this repository.
7. The current baseline passes `cargo check` and the default serial test suite, but all-target/all-feature Clippy fails at `examples/mock_testing.rs:243` on `clippy::useless_borrows_in_formatting`. Release work must clear this pre-existing lint before claiming a green quality gate.

## Priorities and Sequencing

| Priority | Workstream | Why now |
| --- | --- | --- |
| P0 | Release integrity and version recovery | The repository is materially ahead of the existing v0.2.0 tag but still declares 0.2.0. |
| P1 | AUR-aware dependency correctness | This is the highest-value functional gap and the main Pacsea adoption blocker. |
| P1 | Reliability and complexity enforcement | Release and migration confidence depend on deterministic tests and measurable quality gates. |
| P2 | Pacsea compatibility decisions | Localized parser labels and parity behavior should be driven by demonstrated consumer needs. |
| P3 | Optional module enhancements | Useful, but not blockers for releasing or adopting the existing core. |
| External | Pacsea migration | Starts only after a suitable arch-toolkit release is available. |

## Phase 1 — Recover the Release Line (P0)

- [ ] Audit the public API and feature changes between `v0.2.0` and `HEAD`; record any breaking changes and select the next version. Use `0.3.0` unless the audit demonstrates a patch release is semantically correct.
- [ ] Verify whether v0.2.0 was published and whether its GitHub release, tag, and crates.io artifact match commit `83b5c34`; do not infer publication solely from the local tag.
- [ ] Rewrite the changelog with an `Unreleased` section covering index, install, news, sandbox, Pacsea compatibility helpers, dependency updates, fixes, and behavior changes. Correct all placeholder and conflicting dates.
- [ ] Reconcile release automation and `dev/WORKFLOWS/RELEASE_WORKFLOW.md` with the actual version/tag format and available tools; ensure preview mode does not publish, tag, push, or modify external systems.
- [ ] Run the full quality matrix and resolve all failures before changing the version:
  - `cargo fmt --all`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo check`
  - `cargo test -- --test-threads=1`
  - `cargo test --all-features -- --test-threads=1`
  - representative `--no-default-features` feature combinations
- [ ] Set the selected version in `Cargo.toml`/`Cargo.lock`, verify documentation and examples, and run `cargo publish --dry-run`.
- [ ] Request explicit authorization before any real crates.io publish, GitHub release, tag creation, or push.

**Exit criteria:** the version is unique and semantically justified; changelog and release workflow agree; all required checks and package dry-run pass; external release actions remain explicitly approved and auditable.

## Phase 2 — Complete Dependency Resolution (P1)

### 2.1 AUR transitive resolution

- [ ] Define an additive, mockable AUR metadata boundary for `DependencyResolver`; preserve builds that enable `deps` without `aur`.
- [ ] Add failing tests for AUR-only direct dependencies, transitive dependencies, missing metadata, malformed `.SRCINFO`, duplicate dependencies, and helper/network failure.
- [ ] Resolve AUR dependencies through existing `.SRCINFO` fetching/parsing building blocks with explicit depth, node-count, timeout, and concurrency bounds.
- [ ] Add cycle detection and deterministic traversal order; return actionable diagnostics rather than recursing indefinitely or silently dropping nodes.
- [ ] Cache metadata within one resolution run and avoid one subprocess/request per dependency where batching is possible.

### 2.2 Constraint and virtual-package correctness

- [ ] Define deterministic rules for merging compatible version requirements and reporting incompatible ranges.
- [ ] Complete `provides`/`conflicts` handling for official, installed, local, and AUR packages.
- [ ] Preserve dependency provenance so callers can distinguish official, AUR, local, provided, missing, and conflicting nodes.
- [ ] Add unit tests for epochs, pkgrel suffixes, split packages, virtual packages, cycles, and conflicting constraints.
- [ ] Add deterministic integration tests using fixtures/mocks only; live AUR tests remain ignored diagnostics rather than acceptance gates.

### 2.3 Optional presentation/performance

- [ ] Add dependency-tree output only after the graph model is stable; keep rendering separate from resolution.
- [ ] Introduce bounded parallel metadata lookup only after serial correctness tests pass and ordering remains deterministic.

**Exit criteria:** transitive AUR resolution is bounded, cycle-safe, feature-isolated, mock-tested, and compatible with existing resolver entry points; all project checks pass.

## Phase 3 — Reliability and Quality Gates (P1)

- [ ] Run `dev/scripts/complexity_report.sh` (or replace it with a deterministic equivalent) and verify cyclomatic and data-flow complexity are below 25 for every new/touched function.
- [ ] Turn the complexity requirement into a repeatable CI-visible check rather than relying only on Clippy's cognitive-complexity warning.
- [ ] Eliminate or isolate environment-mutation test races; the suite should remain deterministic even though the canonical command uses `--test-threads=1`.
- [ ] Verify graceful degradation when `pacman`, `paru`, `yay`, `sudo`, and `doas` are absent. Error messages must be actionable and query APIs must follow their documented empty/error behavior.
- [ ] Audit all system-modifying command planners for dry-run compatibility at the caller boundary; this library may construct commands but must not silently execute them.
- [ ] Add feature-matrix CI coverage for default, no-default, each standalone module, and all features.
- [ ] Review ignored live/network tests and document which are diagnostics versus release gates.

**Exit criteria:** quality gates are reproducible locally and in CI, feature combinations compile independently, tests are deterministic, and missing system tools do not cause opaque failures.

## Phase 4 — Pacsea Compatibility Decisions (P2)

### 4.1 Localized parser labels

- [ ] First verify a real consumer still passes localized `pacman -Si/-Qi` text into public parsing functions. Internal arch-toolkit commands already force the C locale.
- [ ] If required, design an additive caller-supplied label type and new `*_with_labels` functions while preserving existing one-argument functions unchanged.
- [ ] Keep English labels as fallback, avoid global mutable state and new i18n dependencies, and deduplicate labels efficiently.
- [ ] Add tests for English defaults, German/French examples, mixed labels, multiline fields, localized “None”, and invalid/empty label sets.
- [ ] If no current consumer needs localized raw-output parsing, close this item as unnecessary rather than adding unused API surface.

### 4.2 Behavioral parity contract

- [ ] Freeze and test the Pacsea-facing semantics already added: AUR info percent encoding, repo-aware index fetching, doas-first auto detection, AUR-helper fallback strings, split update commands, `InstallPlan::to_shell_string`, tolerant index loading, batched host queries, and ranked fuzzy search.
- [ ] Explicitly document accepted differences: no implicit `pacman -Sy && pacman -S`, no terminal/password/PAM orchestration, no Pacsea-specific cache/circuit-breaker layer, and graceful query degradation when pacman is unavailable.

**Exit criteria:** compatibility APIs exist only for demonstrated needs, old public signatures remain valid, and parity differences are covered by tests or explicit consumer-side decisions.

## Phase 5 — Optional Capability Batches (P3)

Implement these as independent, separately approved features after P0/P1 work. Each batch requires unit tests, integration tests, structured rustdoc, feature isolation, dry-run handling where relevant, and the full project verification matrix.

### 5.1 News

- [ ] Add HTML-to-text article extraction with safe handling of paragraphs, lists, code, and relative links.
- [ ] Add per-advisory detail fetching only where feed data is insufficient.
- [ ] Add optional feed caching by reusing a generic cache boundary rather than coupling news to AUR internals.

### 5.2 Index and mirrors

- [ ] Design mirror discovery/generation as a portable, optional API; use `reqwest` rather than shelling out to `curl`.
- [ ] Add background index refresh as a returned future or callback-based API with cancellation and explicit error delivery.
- [ ] Keep derivative-distro repository discovery intact and test included `pacman.conf` files with fixtures.

### 5.3 AUR and official metadata

- [ ] Add pagination only where an upstream endpoint supports it and define behavior around the AUR RPC 200-result cap.
- [ ] Add streaming only if it materially reduces memory or latency for a demonstrated caller.
- [ ] Add official repository detail and mirror-health clients behind clearly scoped APIs.
- [ ] Do not pursue WebSocket support; AUR provides no corresponding service.

### 5.4 Sandbox security analysis

- [ ] Threat-model the proposed PKGBUILD lint/risk-scoring scope before implementation; distinguish deterministic static findings from external scanner/reputation results.
- [ ] Use structured findings with rule IDs and evidence rather than an opaque aggregate score.
- [ ] Keep external tools optional, bounded, and gracefully unavailable; never execute PKGBUILD content during analysis.
- [ ] Add malicious/benign fixtures, false-positive tests, and explicit limitations.

**Exit criteria:** each accepted batch solves a demonstrated use case without expanding the default dependency set or weakening safety/feature isolation.

## Phase 6 — Pacsea Migration (External Repository)

This phase is executed and verified in Pacsea, not in `arch-toolkit`. Begin only after the required toolkit version is published or otherwise pinned to an approved immutable revision.

- [ ] Add `arch-toolkit` with only the required features and pass one shared configured `reqwest::Client` to toolkit fetch functions.
- [ ] Replace AUR search, details, comments, and PKGBUILD paths with the corresponding `ArchClient::aur()` APIs.
- [ ] Replace dependency parsing/resolution incrementally; retain consumer-specific locale handling only if Phase 4 proves it necessary.
- [ ] Replace index queries with enabled-repository detection plus Pacsea-specific `repos.conf` additions; preserve Pacsea's enrichment merge.
- [ ] Replace install/update/remove command construction and privilege auto-detection while keeping PTY execution, password handling, hold tails, dry-run wrapping, mirror ranking, lock checks, and app policy in Pacsea.
- [ ] Replace news/advisory fetching while preserving Pacsea caching/circuit breaking; migrate advisory read-state keys deliberately.
- [ ] Replace sandbox dependency analysis while keeping fetching and security scanners in Pacsea; test the intentional version-constraint behavior change.
- [ ] Run parity tests for `--aur`, `--needed`, AUR-helper messages, mixed-install short-circuiting, repository selection, advisory identity, and missing-tool behavior.
- [ ] Remove duplicated Pacsea code only after each cutover passes its acceptance tests and rollback point is recorded.

**Exit criteria:** Pacsea tests and user workflows pass against the released/pinned toolkit, accepted differences are documented in Pacsea, and duplicate code is removed only after verified cutover.

## Global Implementation Rules

For every code change:

1. Reproduce bugs with a failing test before fixing them.
2. Add structured rustdoc (`What`, `Inputs`, `Output`, `Details`) for new functions, methods, structs, and enums.
3. Keep cyclomatic and data-flow complexity below 25 and functions below 150 lines.
4. Use deterministic tests with no unapproved external state or system modification.
5. Preserve `--dry-run` behavior for any system-modifying workflow and degrade gracefully when Arch-specific tools are absent.
6. Run:

```bash
cargo fmt --all
cargo clippy --all-targets --all-features -- -D warnings
cargo check
cargo test -- --test-threads=1
```

Run all-feature and relevant no-default-feature tests for feature work. Update the branch PR record whenever repository changes are made.

## Plan Completion

The in-repository goal is reached when Phases 1–4 are complete, verified, and released as approved. Phase 5 items may remain deferred by explicit decision rather than blocking the core roadmap. Phase 6 is tracked here as the downstream handoff but is completed only by evidence from the Pacsea repository. Once the in-repository goal is reached, record deferred decisions and move this plan to `plans/archive/`.
