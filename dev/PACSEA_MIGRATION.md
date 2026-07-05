# Pacsea Migration Guide — arch-toolkit Regression Fixes

Written 2026-07-05, after the regression evaluation of adopting arch-toolkit
in Pacsea (ROADMAP.md workstreams A2–A5/B2 in `pacsea/dev/IMPROVEMENTS/`).
For each identified regression risk, the better implementation was chosen and
applied to arch-toolkit. This document lists what changed in the toolkit and
what Pacsea must do to use the new implementations.

Verification status: `cargo test --all-features -- --test-threads=1` green
(292 lib + integration + doc tests), clippy pedantic/nursery clean on
all-features, on the no-`aur` feature combo, and on `deps`-only.

---

## 1. Toolkit changes (already applied)

### 1.1 AUR info endpoint percent-encodes names — `src/aur/info.rs`

**Was:** raw names concatenated into `arg[]=...`; a `+` in a package name
(e.g. `libc++`) decodes server-side as a space, silently returning no results.
**Now:** each name goes through `percent_encode` (Pacsea's behavior).

**Pacsea:** nothing to change — `aur::info` now matches
`fetch_aur_versions` (`src/sources/feeds/updates.rs:959`) encoding. Cutover of
the updates feed to `arch_toolkit::aur::info` is now safe on this axis.

### 1.2 reqwest bumped 0.12 → 0.13 — `Cargo.toml`

**Was:** toolkit's `&reqwest::Client` parameters (news, `fetch_srcinfo`) were
reqwest-0.12 types, incompatible with Pacsea's reqwest 0.13.2 client.
**Now:** toolkit builds against reqwest 0.13.

**Pacsea:** pass your existing configured `reqwest::Client` (UA + timeouts)
directly into `fetch_arch_news`, `fetch_security_advisories`, and
`fetch_srcinfo`. Do NOT pass a default `reqwest::Client::new()` — the toolkit
deliberately applies no timeout itself.

### 1.3 Index fetch supports arbitrary repos + pacman.conf discovery — `src/index/fetch.rs`

**Was:** hardcoded `core`/`extra`/`multilib`; derivative-distro repos
(EndeavourOS, CachyOS, Artix, BlackArch, Chaotic-AUR) were silently dropped.
**Now:**

- `fetch_official_index_for_repos(&["core", "extra", "chaotic-aur"])` — sync
- `fetch_official_index_for_repos_async(Vec<String>)` — async, **pacman-only**
  (never falls back to the network API, unlike `fetch_official_index_async`)
- `detect_enabled_repos()` / `detect_enabled_repos_from(path)` — parses
  `[section]` headers from `/etc/pacman.conf` (skips `[options]`, follows
  `Include =` one level with `*` glob), falls back to the default trio when
  the file is unreadable.

**Pacsea:** replace `fetch_official_pkg_names` (`src/index/fetch.rs:95`) with
`detect_enabled_repos()` + Pacsea's own `repos.conf` additions
(`repos_conf_repo_names_for_index_sl`), then call
`fetch_official_index_for_repos_async`. Keep the repos.conf merge in Pacsea —
it is a Pacsea-specific config file. Keep Pacsea's enrichment-preserving merge
logic (`src/index/update.rs:31-70`) as-is; the toolkit fetch intentionally
returns raw `-Sl` data (empty `arch`/`description`).

### 1.4 Privilege auto-detection now prefers doas — `src/install/detect.rs`

**Was:** sudo-first, opposite of Pacsea's `Auto` mode.
**Now:** `detect_privilege_tool()` returns Doas when present, Sudo otherwise
(doas presence signals a deliberate user setup; sudo ships by default).

**Pacsea:** `active_tool()` (`src/logic/privilege.rs`) can delegate its
`Auto` branch to `detect_privilege_tool()`. Explicit `Sudo`/`Doas` config
modes should keep using `is_privilege_tool_available(tool)` to verify the
configured tool. Password piping, credential warmup/invalidation, and
fingerprint/PAM detection stay in Pacsea (out of toolkit scope, by design).

### 1.5 Runtime AUR-helper shell fallback — `src/install/command.rs`

**Was:** only plan-time helper selection (`build_aur_install(helper, ...)`);
Pacsea's terminal installs resolve paru/yay inside the spawned shell.
**Now:**

- `aur_install_shell_fallback(names, options) -> Result<String>` — emits
  Pacsea's exact `(if command -v paru ...; elif command -v yay ...; else
  echo 'No AUR helper (paru/yay) found.'; fi)` body, with the same strict
  name validation as all builders.
- `aur_update_shell_fallback(noconfirm) -> String` — same body for `-Sua`.
- `NO_AUR_HELPER_MESSAGE` const — the exact error string, for tests/matching.

**Pacsea:** replace `aur_install_body` (`src/install/command.rs:40`) with
`aur_install_shell_fallback`, and the system-update AUR if/elif block
(`src/events/modals/system_update.rs:243-273`) with
`aur_update_shell_fallback`. For the PTY executor (same-process PATH),
plan-time `build_aur_install` + `detect_aur_helper()` is equivalent and
gives a real argv.

### 1.6 Update commands: force-sync and AUR-only — `src/install/command.rs`

**Was:** only `build_update_command` (`-Syu`); Pacsea's split update flow
(`pacman -Syu` / `-Syyu` + conditional `helper -Sua`) had no toolkit shape.
**Now:**

- `build_force_sync_update_command(helper, noconfirm)` → `pacman -Syyu ...`
- `build_aur_update_command(helper, noconfirm)` → `paru -Sua ...`
  (never wrap in `with_privilege`)

**Pacsea:** compose the system-update planner
(`src/events/modals/system_update.rs:194-287`) from these builders. Mirror
ranking (`reflector`/`eos-rankmirrors`/...), cache-clean chains, and the
DB-lock guardrail remain Pacsea-side — they are distro/UX policy, not
package-manager plumbing.

### 1.7 `InstallPlan::to_shell_string()` — `src/install/batch.rs`

**Was:** `build_batch_install` returned separate commands; running them
unconditionally in sequence would let the AUR step run after a pacman failure.
**Now:** `plan.to_shell_string()` renders `cmd1 && cmd2` (Pacsea's mixed-batch
semantics). Callers executing `plan.commands` directly must check each exit
status before the next command.

**Pacsea:** `build_batch_install_command` (`src/install/batch.rs:37`) can
build on `build_batch_install(...)` + `plan.to_shell_string()` + Pacsea's
hold-tail/password-pipe wrapping. The `pacman -Sy && pacman -S` version+
reinstall special case (`batch.rs:121-128`) stays in Pacsea for now — it is
intentionally not in the toolkit (implicit `-Sy` before `-S` is a partial-
upgrade hazard; revisit as an explicit opt-in builder if still wanted).

### 1.8 `load_from_disk_or_default` — `src/index/persist.rs`

**Was:** only `load_from_disk` (propagates `Io`/`Json` errors), whereas
Pacsea's startup silently ignores a corrupt/missing cache.
**Now:** `load_from_disk_or_default(path)` returns an empty index on any
error (logged at debug), preserving Pacsea's self-healing startup.

**Pacsea:** use it in `src/app/runtime/init.rs:775` in place of the local
`load_from_disk` (`src/index/persist.rs:17`). The JSON schema is identical
(same `skip_serializing_if`, `name_to_idx` skipped) — existing
`official_index.json` caches load unchanged. Keep sending refresh errors to
`net_err_tx`.

### 1.9 Sandbox preflight batches host queries — `src/sandbox/analyze.rs`, `src/deps/query.rs`

**Was:** `analyze_dependencies` shelled `pacman -Q <name>` + `pacman -Qi
<name>` per dependency — a subprocess storm under Pacsea's parallel
`FuturesUnordered` preflight.
**Now:** two batched calls per analysis, regardless of dependency count:

- `deps::get_installed_versions() -> HashMap<String, String>` — one
  `pacman -Q` (versions revision-stripped, same rule as
  `get_installed_version`)
- `deps::get_foreign_packages() -> HashSet<String>` — one `pacman -Qqm`
  (replaces per-package `pacman -Qi Repository == local` checks)

`analyze_pkgbuild`/`analyze_srcinfo` query once for all four dependency
categories. The lazy `pacman -Qqo` provides-check still runs only for names
missing from the installed set. Public signatures are unchanged.

**Pacsea:** `resolve_sandbox_info_async` (`src/logic/sandbox/mod.rs:189`) can
now call `analyze_srcinfo`/`analyze_pkgbuild` per package without subprocess
blowup. For many packages in one preflight pass, optionally hoist
`get_installed_versions()`/`get_foreign_packages()` — but per-package cost is
already O(1) subprocesses. PKGBUILD security scanning (`pattern.conf`,
clamav/trivy/semgrep/...) stays in Pacsea; the toolkit has none.

### 1.10 Fuzzy search results are ranked — `src/index/query.rs`

**Was:** unsorted matches with a "caller should sort" note.
**Now:** fuzzy results sorted by score (best first), ties by name.
Substring results keep index order.

**Pacsea:** drop any app-side sorting of toolkit fuzzy results, or keep it —
re-sorting an already-sorted vec is harmless.

### 1.11 Housekeeping

- `rust-version = "1.91"` declared in Cargo.toml; README MSRV corrected
  (was "1.70", impossible with edition 2024 + `Duration::from_mins`).
- README install snippets now reference `0.2`.
- Default `ArchClient` user-agent derives from the crate version
  (`arch-toolkit/0.2.0`, was hardcoded `arch-toolkit/0.1.0`).
- Test-only: reqwest-error fixtures now use an invalid proxy URL
  (`aur::utils::mock_reqwest_error`) — invalid-cert client builds no longer
  fail in reqwest 0.13.

---

## 2. Pacsea changes required (checklist)

Dependency declaration (workstream A1):

```toml
arch-toolkit = { version = "0.2", default-features = false,
                 features = ["index", "install", "deps", "news", "sandbox", "fuzzy-search"] }
```

Add `aur` to the feature list only when cutting over `src/sources/{search,comments,pkgbuild}.rs`
(workstream A2); it pulls the full HTTP stack (reqwest/scraper/chrono/lru).

- [ ] **A1**: add the dependency; construct one shared `reqwest::Client` with
  Pacsea's UA and timeouts; pass it to all toolkit fetch functions.
- [ ] **A3 index**: `detect_enabled_repos()` + repos.conf names →
  `fetch_official_index_for_repos_async`; `load_from_disk_or_default` at
  startup; keep the enrichment-preserving merge and key-set-change
  persistence logic local.
- [ ] **A3 install**: cut `src/install/command.rs` / `batch.rs` over to
  `build_pacman_install`, `aur_install_shell_fallback`,
  `build_batch_install` + `plan.to_shell_string()`, `build_remove_command`,
  `build_update_command`, `build_force_sync_update_command`,
  `build_aur_update_command`, `with_privilege`. Keep terminal spawning,
  hold tails, dry-run wrapping, password piping, and the PTY executor local.
- [ ] **A3 privilege**: `Auto` mode → `detect_privilege_tool()`; explicit
  modes → `is_privilege_tool_available`.
- [ ] **A3 deps**: `src/logic/deps/*` → `arch_toolkit::deps` (parsers are
  byte-compatible: operator order, batch size 50, blank-line blocks). Do NOT
  feed locale-formatted pacman output into toolkit parsers — the toolkit
  always runs its own pacman with `LC_ALL=C` and parses English labels only.
  Pacsea's localized-label parsing (`src/logic/deps/parse.rs`) can be
  retired once all pacman invocations go through the toolkit.
- [ ] **A4 news**: `fetch_arch_news(&client, limit, cutoff)` /
  `fetch_security_advisories(&client, limit, cutoff)`. Keep disk caches,
  circuit breaker, and article-content fetching local. **Read-state
  migration required**: toolkit advisory `id` prefers the Atom `<id>`
  element (spec-correct, stable) over the link; Pacsea keyed read-state on
  the link. Either key Pacsea's seen-map on `advisory.url` (unchanged
  behavior) or migrate stored keys once. Severity/packages are now really
  parsed (Pacsea hardcoded `Unknown`/empty) — review UI filters that assumed
  `Unknown`.
- [ ] **B2 sandbox**: `analyze_srcinfo`/`analyze_pkgbuild` replace
  `src/logic/sandbox/{parse,analyze}.rs`; keep .SRCINFO/PKGBUILD fetching
  and security scanning local. Note one intended behavior change: the
  toolkit checks version constraints against the installed version
  (`version_satisfied`), which Pacsea's original never failed.
- [ ] **A5 parity**: verify `--aur` flag presence, `--needed` reinstall
  dropping, and the exact `No AUR helper (paru/yay) found.` message via
  `NO_AUR_HELPER_MESSAGE`.

## 3. Behavior differences that remain (accepted, documented)

- `fetch_official_index_async()` (no-repo-list variant) still falls back to
  the archlinux.org web API when pacman fails; Pacsea should use the
  `_for_repos_async` variant, which never touches the network.
- Toolkit news/advisory fetches have no caching, rate limiting, or circuit
  breaker — Pacsea's layers remain in front. The toolkit's own archlinux.org
  rate limiter applies only to `aur`-feature endpoints and the index API
  fallback.
- `get_installed_version`/`get_installed_versions` strip at the first `-`
  (valid pacman versions contain `-` only before the pkgrel; epoch versions
  like `1:2.3-4` keep the epoch).
- Graceful degradation: installed/explicit/foreign/version queries return
  empty on pacman failure rather than erroring (matches Pacsea's tolerance;
  UIs cannot distinguish "unknown" from "not installed").
- Known toolkit-test flakiness (pre-existing, unrelated to these changes):
  `tests/index_integration.rs` and the env-var client tests mutate
  `PATH`/env concurrently and can fail under parallel test execution; run
  with `--test-threads=1`.
