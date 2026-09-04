#!/usr/bin/env bash
# Same-base byte control for the `prism::api` facade move (design §8.5).
#
# Runs two prism binaries over EVERY checked-in fixture diff (enumerated by
# this script, paired with its repo directory by the layout convention the
# existing tests use) plus a generated poor-parse fixture, with an identical
# invocation matrix and an identical cwd, and compares STDOUT, STDERR and
# EXIT STATUS for every invocation. The facade move must change no behaviour,
# so any difference is a defect in the move -- never in this script or in the
# fixtures.
#
# Matrix per fixture (design §8.5):
#   (a) single algorithms leftflow, absence, contract, echo, membrane,
#       provenance, primitive  x  formats text, json, paper, review, mermaid
#   (b) multi sets `echo,absence,contract` and `absence,contract,primitive`
#       x  the same five formats
#   (c) `--algorithm chop,absence --format json` (an errors[] entry)
#   (d) `--format callers`
#   (e) `--algorithm leftflow --format json --strict-diagrams` on the
#       diagram_snapshot fixture when that fixture has a diff
# Taint is excluded (documented non-byte-stability).
#
# Usage: scripts/phase0-byte-control.sh <base-bin> <branch-bin>
#   JOBS=<n>   parallel invocations (default: 8)
set -uo pipefail

# --- worker mode ----------------------------------------------------------
# Re-entry from xargs: run ONE invocation against both binaries and compare.
if [[ "${1:-}" == "--one" ]]; then
    shift
    id="$1"
    repo="$2"
    diff="$3"
    shift 3
    label="--repo $repo --diff $diff $*"

    b_out="$PBC_OUT/$id.base.out"
    b_err="$PBC_OUT/$id.base.err"
    n_out="$PBC_OUT/$id.branch.out"
    n_err="$PBC_OUT/$id.branch.err"

    "$PBC_BASE" --repo "$repo" --diff "$diff" "$@" >"$b_out" 2>"$b_err"
    b_code=$?
    "$PBC_BRANCH" --repo "$repo" --diff "$diff" "$@" >"$n_out" 2>"$n_err"
    n_code=$?

    ok=1
    cmp -s "$b_out" "$n_out" || ok=0
    cmp -s "$b_err" "$n_err" || ok=0
    [[ "$b_code" == "$n_code" ]] || ok=0

    if [[ $ok == 1 ]]; then
        rm -f "$b_out" "$b_err" "$n_out" "$n_err"
        printf 'OK %s\n' "$label" >>"$PBC_OUT/results.log"
    else
        printf 'DIFF %s [exit %s vs %s; artifacts %s/%s.*]\n' \
            "$label" "$b_code" "$n_code" "$PBC_OUT" "$id" >>"$PBC_OUT/results.log"
    fi
    exit 0
fi

# --- driver mode ----------------------------------------------------------
if [[ $# -ne 2 ]]; then
    echo "usage: $0 <base-bin> <branch-bin>" >&2
    exit 2
fi

abspath() { (cd "$(dirname "$1")" && printf '%s/%s\n' "$PWD" "$(basename "$1")"); }

SELF="$(abspath "$0")"
PBC_BASE="$(abspath "$1")"
PBC_BRANCH="$(abspath "$2")"
REPO_ROOT="$(cd "$(dirname "$SELF")/.." && pwd)"
JOBS="${JOBS:-8}"

for b in "$PBC_BASE" "$PBC_BRANCH"; do
    [[ -x "$b" ]] || {
        echo "not an executable: $b" >&2
        exit 2
    }
done

cd "$REPO_ROOT" || exit 2

PBC_OUT="$(mktemp -d "${TMPDIR:-/tmp}/phase0-byte-control.XXXXXX")"
export PBC_OUT PBC_BASE PBC_BRANCH
: >"$PBC_OUT/results.log"

echo "base   : $PBC_BASE"
echo "branch : $PBC_BRANCH"
echo "cwd    : $REPO_ROOT"
echo "workdir: $PBC_OUT"
echo

# --- fixture enumeration --------------------------------------------------
# A fixture is a (repo dir, diff file) pair. Diff files: *.diff, *.patch, and
# any *.json that is a prism JSON diff spec (contains "file_path"). The repo
# is the diff's own directory, except for a diff that sits directly in
# tests/fixtures/, whose repo is `tests/fixtures/<stem>-source`
# (tests/integration/hapi_regression_test.rs:16-21).
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

# Generated poor-parse fixture: >10% error nodes in the parsed file, with the
# diff landing on the one function that DOES parse.
POOR="$PBC_OUT/poor_parse"
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

# --- invocation matrix ----------------------------------------------------
ALGOS=(leftflow absence contract echo membrane provenance primitive)
FORMATS=(text json paper review mermaid)
MULTI=(echo,absence,contract absence,contract,primitive)

LIST="$PBC_OUT/invocations.txt"
: >"$LIST"
i=0
for p in "${PAIRS[@]}"; do
    repo="${p%%|*}"
    diff="${p##*|}"
    for a in "${ALGOS[@]}"; do
        for f in "${FORMATS[@]}"; do
            echo "$i $repo $diff --algorithm $a --format $f" >>"$LIST"
            i=$((i + 1))
        done
    done
    for a in "${MULTI[@]}"; do
        for f in "${FORMATS[@]}"; do
            echo "$i $repo $diff --algorithm $a --format $f" >>"$LIST"
            i=$((i + 1))
        done
    done
    echo "$i $repo $diff --algorithm chop,absence --format json" >>"$LIST"
    i=$((i + 1))
    echo "$i $repo $diff --format callers" >>"$LIST"
    i=$((i + 1))
done

DIAGRAM_DIFF=""
while IFS= read -r candidate; do
    DIAGRAM_DIFF="$candidate"
    break
done < <(find tests/fixtures/diagram_snapshot -type f \( -name '*.diff' -o -name '*.patch' -o -name 'diff.json' \) | sort)
if [[ -n "$DIAGRAM_DIFF" ]]; then
    echo "$i tests/fixtures/diagram_snapshot $DIAGRAM_DIFF --algorithm leftflow --format json --strict-diagrams" >>"$LIST"
    i=$((i + 1))
else
    echo "NOTE: tests/fixtures/diagram_snapshot has no diff; strict-diagrams invocation skipped"
fi

echo "invocations: $i (x2 binaries)"
echo "running with JOBS=$JOBS ..."
xargs -P "$JOBS" -L 1 "$SELF" --one <"$LIST"

# --- report ---------------------------------------------------------------
ran=$(wc -l <"$PBC_OUT/results.log" | tr -d ' ')
ndiff=$(grep -c '^DIFF ' "$PBC_OUT/results.log")
echo
if [[ "$ran" != "$i" ]]; then
    echo "FATAL: ran $ran invocations, expected $i" >&2
    exit 1
fi
if [[ "$ndiff" != "0" ]]; then
    grep '^DIFF ' "$PBC_OUT/results.log"
    echo
    echo "byte control FAILED: $ndiff of $i invocations differ (artifacts in $PBC_OUT)" >&2
    exit 1
fi
rm -rf "$PBC_OUT"
echo "byte control PASSED: $i/$i invocations identical (stdout, stderr, exit status)"
exit 0
