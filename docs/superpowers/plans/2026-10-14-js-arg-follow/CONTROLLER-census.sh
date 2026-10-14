#!/bin/sh
# Argument-following census on a private corpus. Prints aggregates only (no file names, no
# per-site rows) and removes every intermediate file on exit.
#   sh CONTROLLER-census.sh <corpus-root> [prism-binary]
# Static only: prism indexes the corpus and the pinned TypeScript 5.9.3 checker parses it.
# No corpus code is executed. Run it on a release build of main (default: this worktree's).
set -eu
umask 077
root=${1:?usage: CONTROLLER-census.sh <corpus-root> [prism-binary]}
here=$(cd "$(dirname "$0")" && pwd)
bin=${2:-$here/../../../../target/release/prism}
census=$here/measure/census.cjs
[ -d "$root" ] || { echo "no such corpus root: $root" >&2; exit 2; }
[ -x "$bin" ] || { echo "no prism binary at $bin (cargo build --release on main)" >&2; exit 2; }
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT INT TERM
"$bin" nav --no-cache call-stats --repo "$root" --dump-sites > "$work/sites.jsonl" 2> /dev/null
# --private-root: directory names inside the controller's own corpus are not filtered;
# --aggregate-only: only aggregate.json is written (into $work), never per-site rows.
node --max-old-space-size=12288 "$census" "$root" "$work/sites.jsonl" "$work/out" --aggregate-only --private-root > /dev/null 2> "$work/census.stderr" || {
  echo "census failed; last stderr line:" >&2; tail -n 1 "$work/census.stderr" | cut -c1-200 >&2; exit 1; }
echo "prism binary: $("$bin" --version 2>/dev/null || echo unknown)"
echo "census sha256: $(shasum -a 256 "$census" | cut -d' ' -f1)"
cat "$work/out/aggregate.json"
