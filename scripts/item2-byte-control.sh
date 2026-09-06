#!/usr/bin/env bash
# Item 2 Task 1 byte control (design §5.1, task-1-brief.md Step 10).
#
# Extends scripts/phase0-byte-control.sh (NOT modified by this script) rather
# than replacing it: this script duplicates that script's fixture
# enumeration (find tests/fixtures ... ':125'), its ALGOS list (':153'), its
# FORMATS list (':154'), and its diagram-snapshot special case (':185'), then
# ADDS two invocation kinds the Phase 0 script does not cover:
#
#   - `--format sarif` for every (fixture, algorithm) pair
#   - `prism targets --repo <repo> --diff <diff>` for every fixture
#
# Before Task 5 lands `--resolution`, both run WITHOUT that flag (task-1
# facts: "add ... prism targets invocations with --resolution nominal once
# Task 5 lands the flag (before Task 5 they run without it)").
#
# Item 2 v1 (this task) is label-only: `CpgEdge::DataFlow` gained a payload
# but no legacy serializer prints edge weights, so every invocation here is
# expected to be byte-identical between the Task 0b base binary and the
# item 2 branch binary. STDOUT + STDERR + EXIT STATUS are compared, exactly
# like the Phase 0 script.
#
# Usage: scripts/item2-byte-control.sh <base-bin> <branch-bin>
#   JOBS=<n>   parallel invocations (default: 8)
set -uo pipefail

# --- worker mode ----------------------------------------------------------
# Re-entry from xargs: run ONE invocation (a full, pre-built argv) against
# both binaries and compare. Unlike the Phase 0 script's worker, this one
# takes the COMPLETE argument list as-is (no assumed --repo/--diff prefix),
# because the `targets` subcommand's own args come before its flags in a
# different shape than the default review invocation.
#
# Build-provenance normalization (found empirically running this script,
# controlled against an unrelated clean commit in the same environment
# before being treated as expected rather than a defect -- see
# task-1-report.md "byte control" section): SARIF embeds
# `prism_git_sha`/`prism_build_identity`/`binary_input_dirty`
# (src/output/sarif.rs:251-253) and `targets` embeds `build_identity`
# (src/api/build_info.rs) -- BuildInfo/SarifRunProperties fields that are
# DELIBERATELY sensitive to the exact source tree a binary was built from
# (that is their whole purpose: cache invalidation on source change). Any
# two binaries built from genuinely different commits -- which a base vs.
# branch byte control always compares -- will legitimately differ on these
# fields regardless of whether the change under test is byte-stable. They
# are normalized out here; every other byte is still compared exactly.
normalize() {
    sed -E \
        -e 's/"prism_git_sha": "[^"]*"/"prism_git_sha": "NORMALIZED"/' \
        -e 's/"prism_build_identity": "[^"]*"/"prism_build_identity": "NORMALIZED"/' \
        -e 's/"binary_input_dirty": (true|false)/"binary_input_dirty": NORMALIZED/' \
        -e 's/"build_identity": "[^"]*"/"build_identity": "NORMALIZED"/' \
        -e 's/"git_sha": "[^"]*"/"git_sha": "NORMALIZED"/' \
        "$1"
}

compare_normalized() {
    diff -q <(normalize "$1") <(normalize "$2") >/dev/null
    status=$?
    if [[ $status -ge 2 ]]; then
        diff -q <(normalize "$1") <(normalize "$2") >/dev/null
        status=$?
    fi
    return "$status"
}

if [[ "${1:-}" == "--one" ]]; then
    shift
    id="$1"
    shift
    label="$*"

    b_out="$IBC_OUT/$id.base.out"
    b_err="$IBC_OUT/$id.base.err"
    n_out="$IBC_OUT/$id.branch.out"
    n_err="$IBC_OUT/$id.branch.err"

    "$IBC_BASE" "$@" >"$b_out" 2>"$b_err"
    b_code=$?
    "$IBC_BRANCH" "$@" >"$n_out" 2>"$n_err"
    n_code=$?

    ok=1
    probe_error=0
    compare_normalized "$b_out" "$n_out"
    status=$?
    [[ $status -eq 1 ]] && ok=0
    [[ $status -ge 2 ]] && probe_error=1
    compare_normalized "$b_err" "$n_err"
    status=$?
    [[ $status -eq 1 ]] && ok=0
    [[ $status -ge 2 ]] && probe_error=1
    [[ "$b_code" == "$n_code" ]] || ok=0

    if [[ $probe_error == 1 ]]; then
        printf 'ERROR %s [comparison probe failed after retry; artifacts %s/%s.*]\n' \
            "$label" "$IBC_OUT" "$id" >>"$IBC_OUT/results.log"
    elif [[ $ok == 1 ]]; then
        rm -f "$b_out" "$b_err" "$n_out" "$n_err"
        printf 'OK %s\n' "$label" >>"$IBC_OUT/results.log"
    else
        printf 'DIFF %s [exit %s vs %s; artifacts %s/%s.*]\n' \
            "$label" "$b_code" "$n_code" "$IBC_OUT" "$id" >>"$IBC_OUT/results.log"
    fi
    exit 0
fi

# --- driver mode ------------------------------------------------------------
if [[ $# -ne 2 ]]; then
    echo "usage: $0 <base-bin> <branch-bin>" >&2
    exit 2
fi

abspath() { (cd "$(dirname "$1")" && printf '%s/%s\n' "$PWD" "$(basename "$1")"); }

SELF="$(abspath "$0")"
IBC_BASE="$(abspath "$1")"
IBC_BRANCH="$(abspath "$2")"
REPO_ROOT="$(cd "$(dirname "$SELF")/.." && pwd)"
JOBS="${JOBS:-8}"

for b in "$IBC_BASE" "$IBC_BRANCH"; do
    [[ -x "$b" ]] || {
        echo "not an executable: $b" >&2
        exit 2
    }
done

cd "$REPO_ROOT" || exit 2

IBC_OUT="$(mktemp -d "${TMPDIR:-/tmp}/item2-byte-control.XXXXXX")"
export IBC_OUT IBC_BASE IBC_BRANCH
: >"$IBC_OUT/results.log"

echo "base   : $IBC_BASE"
echo "branch : $IBC_BRANCH"
echo "cwd    : $REPO_ROOT"
echo "workdir: $IBC_OUT"
echo

# --- fixture enumeration (duplicated from scripts/phase0-byte-control.sh:96-139) ---
PAIRS=()
while IFS= read -r d; do
    case "$d" in
    *\ *)
        echo "NOTE: skipping diff with a space in its path: $d"
        continue
        ;;
    esac
    case "$d" in
    *.json) grep -q '"file_path"' "$d" || continue ;;
    esac
    dir="$(dirname "$d")"
    if [[ "$dir" == "tests/fixtures" ]]; then
        stem="$(basename "$d")"
        stem="${stem%.*}"
        repo="tests/fixtures/${stem}-source"
    else
        repo="$dir"
    fi
    if [[ ! -d "$repo" ]]; then
        echo "NOTE: no repo directory for $d (looked for $repo) -- skipped"
        continue
    fi
    PAIRS+=("$repo|$d")
done < <(find tests/fixtures \( -name '*.diff' -o -name '*.patch' -o -name '*.json' \) | sort)

# Generated poor-parse fixture, same shape as the Phase 0 script's.
POOR="$IBC_OUT/poor_parse"
mkdir -p "$POOR"
{
    for _ in $(seq 1 20); do echo "def broken(:"; done
    echo "def good(a):"
    echo "    b = a + 1"
    echo "    return b"
} >"$POOR/broken.py"
printf '{"files":[{"file_path":"broken.py","modify_type":"Modified","diff_lines":[22]}]}\n' \
    >"$POOR/d.json"
PAIRS+=("$POOR|$POOR/d.json")

if [[ ${#PAIRS[@]} -eq 0 ]]; then
    echo "FATAL: no fixture (repo, diff) pairs found under tests/fixtures" >&2
    exit 1
fi

echo "fixtures (${#PAIRS[@]}):"
for p in "${PAIRS[@]}"; do
    echo "  repo=${p%%|*}  diff=${p##*|}"
done
echo

# --- ALGOS (duplicated from scripts/phase0-byte-control.sh:153) ---
# FORMATS (:154, text/json/paper/review/mermaid) is NOT duplicated here: this
# script's own invocations only ever use --format sarif, so a FORMATS loop
# would be dead code. Running scripts/phase0-byte-control.sh separately
# already covers ALGOS x FORMATS; this script extends coverage to sarif and
# `targets` on the SAME fixtures and algorithms, not a re-run of the Phase 0
# matrix.
ALGOS=(leftflow absence contract echo membrane provenance primitive)

# --- diagram case (duplicated from scripts/phase0-byte-control.sh:181-191) ---
DIAGRAM_DIFF=""
while IFS= read -r candidate; do
    DIAGRAM_DIFF="$candidate"
    break
done < <(find tests/fixtures/diagram_snapshot -type f \( -name '*.diff' -o -name '*.patch' -o -name 'diff.json' \) | sort)
if [[ -z "$DIAGRAM_DIFF" ]]; then
    echo "NOTE: tests/fixtures/diagram_snapshot has no diff; strict-diagrams sarif invocation skipped"
fi

# --- invocation matrix: EXTEND, not replace ---------------------------------
# (a) `--format sarif` for every (fixture, algorithm) pair -- the FORMATS
#     list above does not include sarif (Phase 0's own matrix predates it),
#     so this is purely additive.
# (b) `prism targets --repo <repo> --diff <diff>` for every fixture, using
#     TargetsArgs' own default `--algorithm` (no --resolution: not landed
#     until Task 5).
# (c) one `--format sarif --strict-diagrams` invocation on the diagram
#     fixture, mirroring the Phase 0 script's own diagram case.
LIST="$IBC_OUT/invocations.txt"
: >"$LIST"
i=0
for p in "${PAIRS[@]}"; do
    repo="${p%%|*}"
    diff="${p##*|}"
    for a in "${ALGOS[@]}"; do
        echo "$i --repo $repo --diff $diff --algorithm $a --format sarif" >>"$LIST"
        i=$((i + 1))
    done
    echo "$i targets --repo $repo --diff $diff" >>"$LIST"
    i=$((i + 1))
done

if [[ -n "$DIAGRAM_DIFF" ]]; then
    echo "$i --repo tests/fixtures/diagram_snapshot --diff $DIAGRAM_DIFF --algorithm leftflow --format sarif --strict-diagrams" >>"$LIST"
    i=$((i + 1))
fi

# Non-vacuity guard (task-1-brief.md Step 10): an empty matrix must not pass.
if [[ "$i" -eq 0 ]]; then
    echo "FATAL: zero invocations built -- an empty matrix must not pass vacuously" >&2
    exit 1
fi

echo "invocations: $i (x2 binaries)"
echo "running with JOBS=$JOBS ..."
xargs -P "$JOBS" -L 1 "$SELF" --one <"$LIST"

# --- report -----------------------------------------------------------------
ran=$(wc -l <"$IBC_OUT/results.log" | tr -d ' ')
ndiff=$(grep -c '^DIFF ' "$IBC_OUT/results.log")
nerror=$(grep -c '^ERROR ' "$IBC_OUT/results.log")
echo
if [[ "$ran" != "$i" ]]; then
    echo "FATAL: ran $ran invocations, expected $i" >&2
    exit 1
fi
if [[ "$ndiff" != "0" || "$nerror" != "0" ]]; then
    grep -E '^(DIFF|ERROR) ' "$IBC_OUT/results.log"
    echo
    echo "item2 byte control FAILED: $ndiff differ, $nerror errors out of $i invocations (artifacts in $IBC_OUT)" >&2
    exit 1
fi
rm -rf "$IBC_OUT"
echo "item2 byte control PASSED: $i/$i invocations identical (stdout, stderr, exit status)"
exit 0
