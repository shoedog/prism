#!/bin/sh
# Lane A2 (anonymous callables) census on a private corpus. Prints aggregates only (no file
# names, no per-site rows) and removes every intermediate file on exit.
#   sh CONTROLLER-census.sh <corpus-root> <prism-base-binary> [<prism-prototype-binary>]
# base: a release build of main. prototype: a release build of main + measure/prototype.diff
# (anonymous JS/TS callables own their calls); omit it to get the unrecorded counts only.
# Static only: prism indexes the corpus and the pinned TypeScript 5.9.3 checker parses it.
# No corpus code is executed.
set -eu
umask 077
root=${1:?usage: CONTROLLER-census.sh <corpus-root> <prism-base> [<prism-prototype>]}
base=${2:?usage: CONTROLLER-census.sh <corpus-root> <prism-base> [<prism-prototype>]}
proto=${3:-}
here=$(cd "$(dirname "$0")" && pwd)
census=$here/measure/anon-census.cjs
[ -d "$root" ] || { echo "no such corpus root: $root" >&2; exit 2; }
[ -x "$base" ] || { echo "no prism binary at $base" >&2; exit 2; }
[ -z "$proto" ] || [ -x "$proto" ] || { echo "no prism binary at $proto" >&2; exit 2; }
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT INT TERM
timed() { # timed <label> <binary> <out>: dump sites, record wall seconds and max RSS (aggregate)
  /usr/bin/time -l "$2" nav --no-cache call-stats --repo "$root" --dump-sites > "$3" 2> "$work/$1.time" || {
    echo "$1 prism run failed" >&2; exit 1; }
  printf '%s: %s s wall, %s bytes max RSS\n' "$1" "$(awk '/ real /{print $1}' "$work/$1.time")" "$(awk '/maximum resident/{print $1}' "$work/$1.time")"
}
timed base "$base" "$work/base.jsonl"
if [ -n "$proto" ]; then timed prototype "$proto" "$work/proto.jsonl"; p="$work/proto.jsonl"; else p=-; fi
node --max-old-space-size=12288 "$census" "$root" "$work/base.jsonl" "$p" "$work/out" --aggregate-only --private-root > /dev/null 2> "$work/census.stderr" || {
  echo "census failed; last stderr line:" >&2; tail -n 1 "$work/census.stderr" | cut -c1-200 >&2; exit 1; }
echo "base: $("$base" --version 2>/dev/null || echo unknown)"
[ -z "$proto" ] || echo "prototype: $("$proto" --version 2>/dev/null || echo unknown)"
echo "census sha256: $(shasum -a 256 "$census" | cut -d' ' -f1)"
cat "$work/out/aggregate.json"
