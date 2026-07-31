#!/usr/bin/env fish
# release.fish — local verification and explicitly authorized release actions.
#
# What: Verify a release candidate locally or perform the final external release.
#
# Inputs:
# - `--preview` / `--dry-run`: Run local checks and package dry-runs only (default).
# - `--release`: After verification, request an exact confirmation and perform external actions.
# - `version`: Semantic version that must already match Cargo.toml.
#
# Output:
# - Exit 0 after successful verification/release; non-zero on the first failed gate.
#
# Details:
# - Preview never edits files, commits, tags, pushes, creates releases, or publishes.
# - Version/changelog preparation is intentionally separate and reviewable before this script.
# - Release mode never deletes or recreates an existing local or remote tag.

set -g ROOT (realpath (dirname (status filename))/../..)
set -g MODE preview
set -g VERSION ""

function usage
    echo "Usage: release.fish [--preview|--dry-run|--release] VERSION"
    echo "  --preview   run all local verification without external mutations (default)"
    echo "  --release   verify, then tag vVERSION, push that tag, create a GitHub release, and publish"
end

function fail
    echo "error: $argv" >&2
    return 1
end

function run_gate
    set -l label $argv[1]
    set -e argv[1]
    echo
    echo "== $label =="
    command $argv
    or return (fail "$label failed")
end

function manifest_version
    string match -r '^version = "([^" ]+)"' < "$ROOT/Cargo.toml" | string replace -r '^version = "' '' | string replace '"' '' | head -n 1
end

function verify_local
    cd "$ROOT"; or return 1
    run_gate "format" cargo fmt --all -- --check; or return 1
    run_gate "clippy" cargo clippy --all-targets --all-features -- -D warnings; or return 1
    run_gate "check" cargo check; or return 1
    run_gate "default tests" cargo test -- --test-threads=1; or return 1
    run_gate "all-feature tests" cargo test --all-features -- --test-threads=1; or return 1
    run_gate "parallel determinism" cargo test --all-features; or return 1
    run_gate "complexity self-test" bash dev/scripts/complexity_selftest.sh; or return 1
    run_gate "complexity threshold" bash dev/scripts/complexity_report.sh --quiet; or return 1

    for features in deps index install news sandbox index,fuzzy-search aur,cache-disk deps,aur
        run_gate "feature isolation: $features" cargo check --no-default-features --features "$features"; or return 1
    end
    run_gate "minimal feature isolation" cargo check --no-default-features; or return 1
    run_gate "documentation" cargo doc --all-features --no-deps; or return 1
    run_gate "package contents" cargo package --list --allow-dirty; or return 1
    run_gate "publish dry-run" cargo publish --dry-run --allow-dirty; or return 1
end

function perform_release
    cd "$ROOT"; or return 1
    test (git branch --show-current) = main
    or return (fail "release mode requires the main branch")
    test -z (git status --porcelain)
    or return (fail "release mode requires a clean working tree")

    set -l tag "v$VERSION"
    not git rev-parse --verify --quiet "refs/tags/$tag" >/dev/null
    or return (fail "local tag $tag already exists")
    test -z (git ls-remote --tags origin "refs/tags/$tag")
    or return (fail "remote tag $tag already exists")

    echo
    echo "External actions: create/push $tag, create GitHub release, publish arch-toolkit@$VERSION."
    read --prompt-str "Type 'release $VERSION' to authorize: " confirmation
    test "$confirmation" = "release $VERSION"
    or return (fail "release authorization not confirmed")

    run_gate "create signed release tag" git tag -s "$tag" -m "Release $tag"; or return 1
    run_gate "push release tag" git push origin "$tag"; or return 1
    run_gate "create GitHub release" gh release create "$tag" --title "Release $tag" --generate-notes --prerelease; or return 1
    run_gate "publish crates.io package" cargo publish; or return 1
end

for arg in $argv
    switch $arg
        case --preview --dry-run
            set MODE preview
        case --release
            set MODE release
        case -h --help
            usage
            exit 0
        case '-*'
            fail "unknown option: $arg"
            usage
            exit 2
        case '*'
            test -z "$VERSION"; or begin; fail "only one version may be supplied"; exit 2; end
            set VERSION $arg
    end
end

if test -z "$VERSION"
    set VERSION (manifest_version)
end
string match -qr '^[0-9]+\.[0-9]+\.[0-9]+$' -- "$VERSION"
or begin; fail "version must use X.Y.Z format"; exit 2; end

set -l current (manifest_version)
test "$current" = "$VERSION"
or begin; fail "Cargo.toml is $current; prepare and review version $VERSION before release verification"; exit 2; end

verify_local; or exit 1
if test "$MODE" = release
    perform_release; or exit 1
else
    echo
    echo "Preview complete: no files or external systems were modified by this script."
end
