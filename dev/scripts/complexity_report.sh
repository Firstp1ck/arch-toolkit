#!/usr/bin/env bash
#
# Deterministic cyclomatic / data-flow complexity gate for arch-toolkit.
#
# What:
#   Statically analyzes Rust sources, computes cyclomatic and data-flow
#   complexity per function, prints a stable report, and fails when any
#   function exceeds the threshold or when nothing could be analyzed.
#
# Inputs:
#   --threshold N     Maximum allowed complexity (default: 25).
#   --report-file F   Additionally write the report to F (default: stdout only).
#   --top N           Number of functions listed per ranking (default: 10).
#   --quiet           Print only the summary and violations.
#   [paths...]        Files or directories to analyze (default: src).
#
# Output:
#   Report on stdout. Exit 0 only when at least one function was analyzed and every
#   analyzed function is below the threshold. Exit 1 on violations and exit 2 on
#   usage or analysis errors (including "zero functions analyzed").
#
# Details:
#   - Pure bash + awk: no network access, no cargo build, no extra toolchain.
#     The previous implementation shelled out to `cargo test complexity`, which
#     matched zero tests and therefore always reported success.
#   - Cyclomatic complexity = 1 + decision points (`if`, `while`, `for`, `loop`,
#     match arms `=>`, `&&`, `||`, `?` try operator).
#   - Data-flow complexity = `let` bindings + assignments (`=` and compound
#     assignment operators), i.e. the number of value definitions/mutations.
#   - Comments, string literals (including raw strings), and char literals are
#     stripped before counting so text content cannot inflate or deflate scores.
#   - Output ordering is fully deterministic (LC_ALL=C sort with file:line ties).
#
# Usage:
#   bash dev/scripts/complexity_report.sh
#   bash dev/scripts/complexity_report.sh --threshold 25 src
#   bash dev/scripts/complexity_selftest.sh   # self-tests for this analyzer

set -o errexit
set -o nounset
set -o pipefail

THRESHOLD=25
TOP=10
REPORT_FILE=""
QUIET=false
PATHS=()

usage() {
    cat <<'EOF'
Usage: complexity_report.sh [--threshold N] [--top N] [--report-file FILE] [--quiet] [paths...]

Exit codes:
  0  analysis succeeded and every function is below the threshold
  1  at least one function is at or above the threshold
  2  usage error, unreadable input, or zero functions analyzed
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --threshold)
            [[ $# -ge 2 ]] || { echo "error: --threshold requires a value" >&2; exit 2; }
            THRESHOLD="$2"
            shift 2
            ;;
        --top)
            [[ $# -ge 2 ]] || { echo "error: --top requires a value" >&2; exit 2; }
            TOP="$2"
            shift 2
            ;;
        --report-file)
            [[ $# -ge 2 ]] || { echo "error: --report-file requires a value" >&2; exit 2; }
            REPORT_FILE="$2"
            shift 2
            ;;
        --quiet)
            QUIET=true
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        --)
            shift
            while [[ $# -gt 0 ]]; do PATHS+=("$1"); shift; done
            ;;
        -*)
            echo "error: unknown option: $1" >&2
            usage >&2
            exit 2
            ;;
        *)
            PATHS+=("$1")
            shift
            ;;
    esac
done

if ! [[ "$THRESHOLD" =~ ^[0-9]+$ ]] || [[ "$THRESHOLD" -eq 0 ]]; then
    echo "error: --threshold must be a positive integer (got: $THRESHOLD)" >&2
    exit 2
fi
if ! [[ "$TOP" =~ ^[0-9]+$ ]]; then
    echo "error: --top must be a non-negative integer (got: $TOP)" >&2
    exit 2
fi

if [[ ${#PATHS[@]} -eq 0 ]]; then
    REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
    PATHS=("$REPO_ROOT/src")
fi

# Collect the Rust files to analyze, in a stable order.
FILES=()
for path in "${PATHS[@]}"; do
    if [[ -f "$path" ]]; then
        FILES+=("$path")
    elif [[ -d "$path" ]]; then
        while IFS= read -r file; do
            FILES+=("$file")
        done < <(find "$path" -type f -name '*.rs' | LC_ALL=C sort)
    else
        echo "error: no such file or directory: $path" >&2
        exit 2
    fi
done

if [[ ${#FILES[@]} -eq 0 ]]; then
    echo "error: no Rust source files found in: ${PATHS[*]}" >&2
    exit 2
fi

# ============================================================================
# Analysis: emit one "cyclomatic<TAB>dataflow<TAB>file:line<TAB>name" record
# per function.
# ============================================================================
ANALYZER='
function reset_file_state() {
    in_block_comment = 0
    in_raw_string = 0
    raw_hashes = 0
    depth = 0
    in_fn = 0
    pending_fn = 0
}

# Strip comments and literals so only code tokens remain.
function clean(line,   out, i, n, c, c2, j, hashes, closing) {
    out = ""
    n = length(line)
    i = 1
    while (i <= n) {
        c = substr(line, i, 1)
        if (in_raw_string) {
            if (c == "\"") {
                hashes = 0
                while (substr(line, i + 1 + hashes, 1) == "#") hashes++
                if (hashes >= raw_hashes) {
                    in_raw_string = 0
                    i = i + 1 + raw_hashes
                    out = out "RAWSTR"
                    continue
                }
            }
            i++
            continue
        }
        if (in_block_comment) {
            if (c == "*" && substr(line, i + 1, 1) == "/") {
                in_block_comment = 0
                i += 2
                continue
            }
            i++
            continue
        }
        if (c == "/" && substr(line, i + 1, 1) == "/") break
        if (c == "/" && substr(line, i + 1, 1) == "*") {
            in_block_comment = 1
            i += 2
            continue
        }
        # Raw string: r"..." / r#"..."# / br#"..."#
        if ((c == "r" || c == "b") && substr(line, i, 2) == "br") {
            j = i + 2
        } else if (c == "r") {
            j = i + 1
        } else {
            j = 0
        }
        if (j > 0) {
            hashes = 0
            while (substr(line, j + hashes, 1) == "#") hashes++
            if (substr(line, j + hashes, 1) == "\"") {
                raw_hashes = hashes
                in_raw_string = 1
                i = j + hashes + 1
                continue
            }
        }
        if (c == "\"") {
            i++
            while (i <= n) {
                c2 = substr(line, i, 1)
                if (c2 == "\\") { i += 2; continue }
                if (c2 == "\"") { i++; break }
                i++
            }
            out = out "STR"
            continue
        }
        if (c == "'"'"'") {
            # Escaped char literal, plain char literal, or a lifetime.
            if (substr(line, i + 1, 1) == "\\") {
                j = i + 2
                while (j <= n && substr(line, j, 1) != "'"'"'") j++
                i = j + 1
                out = out "CHR"
                continue
            }
            if (substr(line, i + 2, 1) == "'"'"'") {
                i += 3
                out = out "CHR"
                continue
            }
            out = out " "
            i++
            continue
        }
        out = out c
        i++
    }
    return out
}

function count_occurrences(text, pattern,   n, tmp) {
    n = 0
    tmp = text
    while (match(tmp, pattern)) {
        n++
        tmp = substr(tmp, RSTART + RLENGTH)
    }
    return n
}

# Assignments: "=" that is not part of ==, !=, <=, >=, =>.
function count_assignments(text,   i, n, c, prev_ch, next_ch, total) {
    total = 0
    n = length(text)
    for (i = 1; i <= n; i++) {
        c = substr(text, i, 1)
        if (c != "=") continue
        prev_ch = (i > 1) ? substr(text, i - 1, 1) : ""
        next_ch = substr(text, i + 1, 1)
        if (next_ch == "=") { i++; continue }       # ==
        if (prev_ch == "=" || prev_ch == "!" || prev_ch == "<" || prev_ch == ">") continue
        if (next_ch == ">") { i++; continue }       # =>
        total++
    }
    return total
}

function count_decisions(text,   total, padded) {
    # Pad so keywords at the very start/end still see a delimiter.
    padded = " " text " "
    total = 0
    total += count_occurrences(padded, "[^A-Za-z0-9_]if[^A-Za-z0-9_]")
    total += count_occurrences(padded, "[^A-Za-z0-9_]while[^A-Za-z0-9_]")
    total += count_occurrences(padded, "[^A-Za-z0-9_]for[^A-Za-z0-9_]")
    total += count_occurrences(padded, "[^A-Za-z0-9_]loop[^A-Za-z0-9_]")
    total += count_occurrences(padded, "=>")
    total += count_occurrences(padded, "&&")
    total += count_occurrences(padded, "\\|\\|")
    return total
}

# Data flow: mutable state a reader must track, i.e. `let mut` bindings plus
# every assignment that is not the initializer of a `let` binding. Immutable
# single-assignment `let` chains stay linear and are deliberately not counted,
# so the metric measures state complexity instead of function length.
function count_dataflow(text,   padded) {
    padded = " " text " "
    return count_occurrences(padded, "[^A-Za-z0-9_]let[ \t]+mut[^A-Za-z0-9_]") \
        + count_assignments(text) \
        - count_occurrences(padded, "[^A-Za-z0-9_]let[^;=]*=")
}

function braces(text, ch,   i, n, total) {
    total = 0
    n = length(text)
    for (i = 1; i <= n; i++) if (substr(text, i, 1) == ch) total++
    return total
}

function emit() {
    printf "%d\t%d\t%s:%d\t%s\n", cyclo, dataflow, fn_file, fn_line, fn_name
    analyzed++
}

FNR == 1 { reset_file_state() }

{
    code = clean($0)

    if (in_fn) {
        cyclo += count_decisions(code)
        dataflow += count_dataflow(code)
        depth += braces(code, "{") - braces(code, "}")
        if (depth <= 0) {
            emit()
            in_fn = 0
            depth = 0
        }
        next
    }

    if (!pending_fn && match(code, /(^|[^A-Za-z0-9_])fn[ \t]+[A-Za-z_][A-Za-z0-9_]*/)) {
        sig = substr(code, RSTART, RLENGTH)
        sub(/^[^A-Za-z_]*/, "", sig)
        sub(/^fn[ \t]+/, "", sig)
        fn_name = sig
        fn_file = FILENAME
        fn_line = FNR
        pending_fn = 1
        # Only consider the part of the line at/after the signature.
        code = substr(code, RSTART)
    }

    if (pending_fn) {
        opens = braces(code, "{")
        if (opens > 0) {
            pending_fn = 0
            in_fn = 1
            cyclo = 1
            dataflow = 0
            depth = 0
            cyclo += count_decisions(code)
            dataflow += count_dataflow(code)
            depth += opens - braces(code, "}")
            if (depth <= 0) {
                emit()
                in_fn = 0
                depth = 0
            }
        } else if (index(code, ";") > 0) {
            # Trait method declaration or function pointer type: no body.
            pending_fn = 0
        }
    }
}

END { if (analyzed == 0) exit 3 }
'

RECORDS_FILE="$(mktemp)"
SORTED_FILE="$(mktemp)"
trap 'rm -f "$RECORDS_FILE" "$SORTED_FILE"' EXIT

set +o errexit
LC_ALL=C awk "$ANALYZER" "${FILES[@]}" >"$RECORDS_FILE"
awk_status=$?
set -o errexit

if [[ $awk_status -eq 3 ]]; then
    echo "error: complexity analysis found zero functions in ${#FILES[@]} file(s); refusing to report success" >&2
    exit 2
fi
if [[ $awk_status -ne 0 ]]; then
    echo "error: complexity analysis failed (awk exit $awk_status)" >&2
    exit 2
fi

ANALYZED=$(wc -l <"$RECORDS_FILE" | tr -d '[:space:]')
if [[ "$ANALYZED" -eq 0 ]]; then
    echo "error: complexity analysis produced no records; refusing to report success" >&2
    exit 2
fi

# ============================================================================
# Report
# ============================================================================
{
    echo "=== Complexity Report (threshold: $THRESHOLD) ==="
    echo "Files analyzed:     ${#FILES[@]}"
    echo "Functions analyzed: $ANALYZED"
    echo

    if [[ "$QUIET" != true && "$TOP" -gt 0 ]]; then
        echo "=== Top $TOP Most Complex Functions (Cyclomatic) ==="
        LC_ALL=C sort -t$'\t' -k1,1nr -k3,3 -k4,4 "$RECORDS_FILE" |
            awk -F'\t' -v top="$TOP" \
                'NR <= top { printf "%d. cyclomatic=%s dataflow=%s %s %s\n", NR, $1, $2, $3, $4 }'
        echo

        echo "=== Top $TOP Most Complex Functions (Data Flow) ==="
        LC_ALL=C sort -t$'\t' -k2,2nr -k3,3 -k4,4 "$RECORDS_FILE" |
            awk -F'\t' -v top="$TOP" \
                'NR <= top { printf "%d. dataflow=%s cyclomatic=%s %s %s\n", NR, $2, $1, $3, $4 }'
        echo
    fi

    LC_ALL=C awk -F'\t' -v threshold="$THRESHOLD" \
        '$1 >= threshold || $2 >= threshold' "$RECORDS_FILE" |
        LC_ALL=C sort -t$'\t' -k3,3 -k4,4 >"$SORTED_FILE"

    violations=$(wc -l <"$SORTED_FILE" | tr -d '[:space:]')
    echo "=== Threshold Violations (>= $THRESHOLD) ==="
    if [[ "$violations" -eq 0 ]]; then
        echo "none"
    else
        LC_ALL=C awk -F'\t' '{ printf "%s %s: cyclomatic=%s dataflow=%s\n", $3, $4, $1, $2 }' "$SORTED_FILE"
    fi
    echo
    echo "Violations: $violations"
} | if [[ -n "$REPORT_FILE" ]]; then tee "$REPORT_FILE"; else cat; fi

violations=$(LC_ALL=C awk -F'\t' -v threshold="$THRESHOLD" \
    'BEGIN { n = 0 } $1 >= threshold || $2 >= threshold { n++ } END { print n }' "$RECORDS_FILE")

if [[ "$violations" -ne 0 ]]; then
    echo "error: $violations function(s) reached the complexity threshold of $THRESHOLD" >&2
    exit 1
fi

exit 0
