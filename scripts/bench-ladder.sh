#!/bin/bash
# Prism scale-ladder benchmark (S1 spec §6). Usage:
#   scripts/bench-ladder.sh [--cache-dir DIR] [--timeout SECS] [name:path ...]
# Defaults: fresh temp cache dir; 2400s; pinned list prism,tokio,hugo,django,rust-analyzer.
# prism resolves to this checkout; others under $PRISM_BENCH_REPOS (default ~/code/bench-repos).
# Absent repo paths emit a `missing` row, never an error.
# Emits one markdown row per repo:
#   repo | loc | files | cold_s | maxrss_mb | cache_mb | warm_s | status
# cold  = first run against an empty cache subdir (CPG build + cache write)
# warm  = the identical command repeated immediately (cache hit)
# Requires GNU timeout (brew coreutils) and macOS /usr/bin/time -l (RSS in bytes -> MB).
set -u
command -v timeout >/dev/null || { echo "needs GNU timeout (brew install coreutils)" >&2; exit 2; }
/usr/bin/time -l true 2>/dev/null || { echo "needs BSD /usr/bin/time -l (macOS)" >&2; exit 2; }
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PRISM="$ROOT/target/release/prism"
BENCH_REPOS="${PRISM_BENCH_REPOS:-$HOME/code/bench-repos}"
CACHE_BASE="$(mktemp -d /tmp/prism-bench-cache.XXXXXX)"; TMO=2400; REPOS=()
while [ $# -gt 0 ]; do case "$1" in
  --cache-dir) CACHE_BASE="$2"; shift 2;; --timeout) TMO="$2"; shift 2;;
  *) REPOS+=("$1"); shift;; esac; done
if [ ${#REPOS[@]} -eq 0 ]; then REPOS=(
  "prism:$ROOT" "tokio:$BENCH_REPOS/tokio" "hugo:$BENCH_REPOS/hugo"
  "django:$BENCH_REPOS/django" "rust-analyzer:$BENCH_REPOS/rust-analyzer" ); fi
echo "repo | loc | files | cold_s | maxrss_mb | cache_mb | warm_s | status"
EXT=('*.rs' '*.go' '*.py' '*.js' '*.jsx' '*.ts' '*.tsx' '*.c' '*.cc' '*.cpp' '*.h' '*.hpp' '*.java' '*.lua' '*.tf' '*.sh' '*.bash')
for spec in "${REPOS[@]}"; do
  name="${spec%%:*}"; repo="${spec#*:}"; cdir="$CACHE_BASE/$name"; mkdir -p "$cdir"
  if [ ! -d "$repo" ]; then
    echo "$name | - | - | - | - | - | - | missing ($repo)"; continue
  fi
  loc=$(cd "$repo" && git ls-files -- "${EXT[@]}" 2>/dev/null \
    | grep -vE '^(vendor|node_modules|dist|build|target)/' | tr '\n' '\0' \
    | xargs -0 cat 2>/dev/null | wc -l | tr -d ' ')
  files=$(cd "$repo" && git ls-files -- "${EXT[@]}" 2>/dev/null \
    | grep -cvE '^(vendor|node_modules|dist|build|target)/')
  t0=$(date +%s)
  /usr/bin/time -l timeout "$TMO" "$PRISM" nav --cache-dir "$cdir" repo-map \
    --repo "$repo" --format json >/dev/null 2>"/tmp/bench-$name.time"
  st=$?; t1=$(date +%s); cold=$((t1 - t0))
  rss=$(awk '/maximum resident set size/{printf "%.0f", $1/1048576}' "/tmp/bench-$name.time")
  if [ $st -eq 124 ]; then
    echo "$name | $loc | $files | TIMEOUT>${TMO}s | ${rss:-?} | - | - | timeout"; continue
  elif [ $st -ne 0 ]; then
    echo "$name | $loc | $files | $cold | ${rss:-?} | - | - | exit$st"; continue
  fi
  cmb=$(du -sm "$cdir" 2>/dev/null | awk '{print $1}')
  w0=$(python3 -c 'import time; print(time.time())')
  timeout 300 "$PRISM" nav --cache-dir "$cdir" repo-map --repo "$repo" --format json >/dev/null 2>&1
  wst=$?
  w1=$(python3 -c 'import time; print(time.time())')
  warm=$(python3 -c "print(f'{$w1 - $w0:.2f}')")
  if [ $wst -eq 124 ]; then warm="TIMEOUT"; elif [ $wst -ne 0 ]; then warm="exit$wst"; fi
  echo "$name | $loc | $files | $cold | ${rss:-?} | $cmb | $warm | ok"
done
