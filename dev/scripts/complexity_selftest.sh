#!/usr/bin/env bash
#
# What: Verify the deterministic Rust complexity gate's pass, violation, and empty-input paths.
#
# Inputs:
# - No arguments; temporary Rust fixtures are generated locally.
#
# Output:
# - Exit 0 when all analyzer contracts hold, non-zero otherwise.
#
# Details:
# - Uses no network or repository mutation beyond temporary files.
# - Proves the gate rejects threshold violations and zero analyzed functions.

set -o errexit
set -o nounset
set -o pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ANALYZER="$SCRIPT_DIR/complexity_report.sh"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

cat >"$TMP_DIR/pass.rs" <<'RUST'
fn simple(value: bool) -> usize {
    if value { 1 } else { 0 }
}
RUST

cat >"$TMP_DIR/fail.rs" <<'RUST'
fn branchy(a: bool, b: bool, c: bool) -> usize {
    if a && b || c { 1 } else { 0 }
}
RUST

printf 'const VALUE: usize = 1;\n' >"$TMP_DIR/empty.rs"

bash "$ANALYZER" --quiet --threshold 5 "$TMP_DIR/pass.rs" >/dev/null
if bash "$ANALYZER" --quiet --threshold 3 "$TMP_DIR/fail.rs" >/dev/null 2>&1; then
    echo "error: violation fixture unexpectedly passed" >&2
    exit 1
fi
set +o errexit
bash "$ANALYZER" --quiet "$TMP_DIR/empty.rs" >/dev/null 2>&1
empty_status=$?
set -o errexit
if [[ $empty_status -ne 2 ]]; then
    echo "error: zero-function fixture returned $empty_status instead of 2" >&2
    exit 1
fi

echo "complexity analyzer self-test passed"
