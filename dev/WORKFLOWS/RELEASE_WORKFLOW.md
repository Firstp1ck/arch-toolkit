# Release Workflow

Automated release process using `release.fish`.

## Usage

```fish
# Full release
./dev/scripts/release.fish 0.1.0

# Preview (dry-run)
./dev/scripts/release.fish --dry-run 0.1.0
```

## Pre-flight Checks

Before starting, the script verifies:
- ✅ On `main` branch
- ✅ Working directory is clean (no uncommitted changes)

## Workflow Steps

### Phase 1: Version Update
1. Update `Cargo.toml` version
2. Run `cargo check` to update `Cargo.lock`

### Phase 2: Documentation
3. **[Cursor]** Run `/release-new {version}` → creates `Documents/RELEASE_v{version}.md`
4. Update `CHANGELOG.md` with release notes
5. **[Cursor]** Run `/readme-update` (if needed)

### Phase 3: Build and Test
6. Run `cargo-dev` (tests/checks)
7. Build library with all features
8. Run `cargo publish --dry-run` to verify

### Phase 4: Release
9. Commit and push all changes
10. Create git tag (tag = version, e.g., `0.1.0`)
11. Push tag to GitHub
12. Create GitHub release
13. Publish to crates.io: `cargo publish`

## Manual Steps (Cursor AI)

The script pauses at these steps for you to run Cursor commands:

| Step | Command | Output |
|------|---------|--------|
| 3 | `/release-new {version}` | `Documents/RELEASE_v{version}.md` |
| 5 | `/readme-update` | Updates `README.md` |

## Files Updated

| File | Description |
|------|-------------|
| `Cargo.toml` | Version number |
| `Cargo.lock` | Dependency lock |
| `Documents/RELEASE_v{version}.md` | Release notes |
| `CHANGELOG.md` | Cumulative changelog |
| `README.md` | Project readme |

## Prerequisites

- `cursor` CLI (for documentation generation)
- `gh` CLI (GitHub)
- `cargo` with crates.io credentials configured
- `git`
- Fish functions: `cargo-dev`

## Crates.io Publishing

Before publishing, ensure:
- You have a crates.io account
- You're added as an owner/maintainer of the crate
- Your crates.io token is configured: `cargo login <token>`
- Run `cargo publish --dry-run` to verify everything works
