# Release Workflow

`dev/scripts/release.fish` separates local verification from external release actions.

## Prepare

1. Audit public API changes and select a unique semantic version.
2. Update `Cargo.toml`, `Cargo.lock`, and `CHANGELOG.md` in a reviewed change.
3. Keep the canonical tag format `v<version>` (for example, `v0.3.0`).
4. Ensure required tools are available: Rust/Cargo, Bash, Fish, Git, and—only for release mode—`gh` plus signing credentials.

## Preview (no external side effects)

```fish
./dev/scripts/release.fish --preview 0.3.0
# --dry-run is an alias for --preview
```

Preview does not edit files, commit, tag, push, create a GitHub release, or publish. It runs:

- formatting, all-target/all-feature Clippy, check, default tests;
- serial and parallel all-feature tests;
- complexity analyzer self-tests and the threshold-25 gate;
- minimal and representative feature-isolation checks;
- rustdoc, package-content inspection, and `cargo publish --dry-run --allow-dirty`.

A failed check stops the preview. Preview never reports a skipped check as successful.

## Release (explicit external authorization)

Real release actions require separate user authorization. After approval and from a clean `main` worktree:

```fish
./dev/scripts/release.fish --release 0.3.0
```

The script reruns the full preview, refuses existing local/remote tags, displays the exact external actions, and requires typing `release 0.3.0`. It then:

1. creates signed tag `v0.3.0`;
2. pushes only that tag;
3. creates a prerelease GitHub release with generated notes;
4. runs `cargo publish`.

It never deletes/recreates tags and never commits or pushes branch changes. If a later external step fails after the tag was pushed, stop and reconcile manually; do not rewrite published history.

## Live diagnostic tests

Tests marked `#[ignore]` because they require AUR/network access, pacman databases, or host package state are diagnostics, not release gates. Run them manually only in a controlled Arch environment:

```bash
cargo test --all-features -- --ignored --test-threads=1
```

Deterministic fixtures/mocks and the normal CI matrix are the release acceptance evidence.
