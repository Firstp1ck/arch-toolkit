# Open Tasks

Consolidated open work for arch-toolkit, gathered from the completed phase plan
documents (2026-07-05). All six planned modules — `aur`, `deps`, `index`,
`install`, `news`, `sandbox` — are core-complete; the completed phase docs
(`*_MODULE_PHASE.md`, `AUR_TOOLKIT_CRATE_PREPARATION.md`,
`FRAMEWORK_AGNOSTIC_FILES.md`) were removed and remain available in git history.

---

## 1. Release (next actionable step)

- [ ] **Publish v0.2.0 to crates.io**
  - All modules green: fmt/clippy clean across all feature combos, full test
    suite passes with `cargo test -- --test-threads=1`
  - Version in `Cargo.toml` is already `0.2.0`
- [x] **Update README version strings** — install snippets unified on `0.2`;
  MSRV corrected to 1.91 and `rust-version` declared in Cargo.toml (2026-07-05)
- [ ] **Write changelog** for v0.2.0 (deps completion, index, install, news,
  sandbox modules; `ArchToolkitError::Io` variant; aur-gated error variants;
  multi-line PKGBUILD array parser fix)

## 2. Deps Module

### High value

- [ ] **AUR dependency queries in the resolver** (from Pacsea `src/logic/deps/aur.rs`)
  - Async .SRCINFO fetching inside `DependencyResolver` so AUR dependencies
    resolve transitively; known limitation documented since Phase 2
  - Building blocks already exist: `deps::fetch_srcinfo` (aur feature) +
    `sandbox::analyze_srcinfo` show the pattern

### Quality

- [ ] **Verify cyclomatic complexity < 25 for all functions** — the one
  unchecked Phase 2 acceptance criterion; never measured (clippy
  `cognitive_complexity` is set to `warn` and currently silent, so this may
  just need a measurement pass and a checkbox)

### Future enhancements (deferred from Phase 2)

- [ ] Dependency tree visualization — tree structure for deps
- [ ] Cycle detection — detect circular dependencies
- [ ] Version range merging — combine version requirements
- [ ] AUR deep resolution — full AUR dependency tree
- [ ] Provides/Conflicts resolution — full virtual package handling
- [ ] Parallel resolution — async batch queries

## 3. Index Module (optional tasks from Phase 3)

- [ ] **Task 3.5: Mirror management** (medium priority, Windows-specific, ~2-3h)
  - Port `fetch_mirrors()` / `generate_mirrorlist()` from Pacsea `src/index/mirrors.rs`
  - Replace curl with reqwest; define `MirrorInfo` type; make Windows-specific
    code optional (cfg/feature)
- [ ] **Task 3.6: Background index updates** (low priority, ~2-3h)
  - Port `update_in_background()` from Pacsea `src/index/update.rs`
  - No Pacsea notification channels — use callback or returned future

## 4. News Module (future enhancements from Phase 5)

- [ ] **Article content extraction** — HTML → text rendering from Pacsea
  `news/parse.rs` (scraper-based; paragraphs, bullets, code markers, link resolution)
- [ ] **Per-advisory detail fetching** — severity/packages from advisory pages
  (the feed's `<content>` block already covers most cases)
- [ ] **Optional caching integration** — reuse the aur module's cache layer for feeds

## 5. AUR Module (medium/low priority from Phase 1 follow-ups)

- [ ] **Pagination support** — AUR RPC caps at 200 results (usually sufficient)
- [ ] **Streaming support** — return streams for large responses
- [ ] **Official repo details** — fetch package details from archlinux.org
- [ ] **Mirror status** — fetch mirror health information
- [ ] ~~WebSocket support~~ — moot; AUR does not offer it

## 6. Sandbox Module (future enhancement from Phase 6)

- [ ] **PKGBUILD security linting / risk scoring** — the originally-envisioned
  "security analysis" never existed in Pacsea (its security angle is external
  aur-sleuth/VirusTotal tooling); could be built on top of the existing
  dependency-delta analysis

## 7. Pacsea Migration (work in the Pacsea repo, not here)

Once v0.2.0 is published, Pacsea can migrate incrementally:

- [ ] Add `arch-toolkit` dependency with the needed features
- [ ] Replace `src/sources/search.rs` → `ArchClient::aur().search()`
- [ ] Replace `src/sources/details.rs` (AUR parts) → `ArchClient::aur().info()`
- [ ] Replace `src/sources/comments.rs` → `ArchClient::aur().comments()`
- [ ] Replace `src/sources/pkgbuild.rs` → `ArchClient::aur().pkgbuild()`
- [ ] Replace `src/logic/deps/` parsing/resolution → `arch_toolkit::deps`
- [ ] Replace `src/index/` queries → `arch_toolkit::index`
- [ ] Replace install command building → `arch_toolkit::install`
- [ ] Replace news/advisories fetching → `arch_toolkit::news`
- [ ] Replace sandbox preflight → `arch_toolkit::sandbox`
- [ ] Remove duplicated code from Pacsea

---

## Module Status Reference

| Module | Feature | Status | Standalone build |
|--------|---------|--------|------------------|
| AUR | `aur` (default) | ✅ Complete (v0.1.x, published) | ✅ |
| Dependencies | `deps` | ✅ Complete | ✅ |
| Index | `index` | ✅ Core complete (3.5/3.6 optional, open) | ✅ |
| Install | `install` | ✅ Complete | ✅ |
| News | `news` | ✅ Complete | ✅ |
| Sandbox | `sandbox` | ✅ Complete | ✅ |

Historical phase plans (in git history): `AUR_TOOLKIT_CRATE_PREPARATION.md`,
`DEPENDENCIES_MODULE_PHASE.md`, `INDEX_MODULE_PHASE.md`,
`INSTALL_MODULE_PHASE.md`, `NEWS_MODULE_PHASE.md`, `SANDBOX_MODULE_PHASE.md`,
`FRAMEWORK_AGNOSTIC_FILES.md`.
