# DFG compaction PR E — preparation and controller acceptance

Status: **NOT MEASURED; activation acceptance pending.** The worker prepares
Task 6 and Task 7 tooling/binaries. Only the controller runs the real corpora.
No memory, load, peak or physical-residency PASS is asserted here.

The activation base is `e5a45634d34b9a543986a0b59e107634df486485` (A–D plus
intervening main). Cache 120 becomes 121 exactly once; navigation sidecar 66
stays unchanged. Both 132-case manifests reproduced clean main unchanged.
The controller must make the entire activation one commit, after the separate
re-pin commit, and bind the reviewed activation SHA and its exact first parent.
A documentation-only re-pin parent has the same binary inputs as e5a45634.

## Prepared artifacts and identity binding

Worker evidence lives in `dfgE-scratch/` adjacent to the implementation checkout:
`bin/{base,candidate}/` contains production replay binaries;
`bin/probe-{base,candidate}/` contains isolated instrumented release `prism` and
`prism-mcp` binaries, built with `mcp` and without `dfg-oracle`.
`binary-receipts.json` and `probe-binary-receipts.json` bind binary SHA256,
source manifests/dirty patches, compiler, profile, grammar, cache and build IDs.
`probe-instrumentation.patch` is the original `c3162f94` 14-file reference.
The ports move DFG census access into scratch-only `data_flow/probe_census.rs`
and add identical query timers. They are excluded from production commits.

The prepared probe ports use the **matched dirty override**, with read-only Git
metadata bound to the source base; they contain no worker commits. Before corpus
measurement, the controller must bind `DFG_CANDIDATE_SHA` to the reviewed Task 6
commit and `DFG_BASE_SHA` to `git rev-parse "$DFG_CANDIDATE_SHA^"`, compare the
production bytes with these snapshots, and rebuild/re-receipt the isolated ports
on those exact source SHAs. The worker binaries are validated preparation artifacts;
the reviewed activation commit does not yet exist in this uncommitted lane. Do not label e5a45634 or a later receipt
commit as the reviewed activation SHA. Prefer clean committed probe ports; if
using these dirty ports, retain both patches, source manifests and explicit
`binary_input_dirty=true` receipts. This binding is a prerequisite, not acceptance.

Copy the binaries and receipts without overwriting earlier evidence to
`~/prism-evidence/mcp-memory/dfg-compaction-2026-10-14/bin/{base,candidate}/`.
Copy `dfgE-scratch/summarize_probe.py` alongside the evidence root. Tiny preflight
receipts must show one cold prime and two warm CPG + call-edge sidecar hits for
both builds, probe meta hits and no census rows. Repeat tiny preflight after any
rebuild or clean/dirty-mode change, before spending a corpus set.

## Pre-registered controller procedure

Reserve the quiet host and exclusive process slot. At every boundary capture
`pgrep -fl 'prism-mcp|cargo|rustc|nextest|mutgate|memreport.py'`, uptime, swap and
`RAYON_NUM_THREADS`; wait if another build/index/benchmark is active. Never
continue because a quiet-wait timeout expired. Do not kill unrelated work.
Record both corpus commits, clean statuses and workload MANIFEST/file SHA256s
before running and confirm them afterwards. Corpora are
`~/code/bench-repos/prometheus` and `~/code/prism-c1-corpus`; workloads are
`~/prism-evidence/mcp-memory/c1/workloads`. Keep toolchain/profile/allocator,
settings, paths and environment matched; unset `RAYON_NUM_THREADS` for both.

Bind `DFG_EVIDENCE=$HOME/prism-evidence/mcp-memory/dfg-compaction-2026-10-14`.
Create a fresh `set-1` (must not exist). Export `PRISM_PROBE_NO_CENSUS=1`.
For clean committed ports unset `PRISM_NAV_EDGE_CACHE_LOAD_DIRTY`; for the prepared
dirty ports export `PRISM_NAV_EDGE_CACHE_LOAD_DIRTY=1` for **both**, including tiny
preflight, primes and warms. Record the choice in both receipts.

Run the following command four times, sequentially, in fixed order:
`base/prometheus`, `candidate/prometheus`, `base/slicing`, `candidate/slicing`.
Set `SIDE`, `CORPUS`, `CORPUS_PATH` for that row. Each invocation owns a fresh
output directory and runs one cache-writing cold prime plus two cache hits:

```sh
nice -n 19 env PRISM_PROBE_OUT="$DFG_EVIDENCE/set-1/$SIDE-$CORPUS.jsonl" \
  python3 scripts/memreport/memreport.py run \
  --bin "$DFG_EVIDENCE/bin/$SIDE/prism-mcp" \
  --corpus "$CORPUS=$CORPUS_PATH" \
  --workloads "$HOME/prism-evidence/mcp-memory/c1/workloads" \
  --kinds warm --reps 2 --out "$DFG_EVIDENCE/set-1/$SIDE-$CORPUS" \
  > "$DFG_EVIDENCE/set-1/$SIDE-$CORPUS.log" 2>&1
cat "$DFG_EVIDENCE/set-1/$SIDE-$CORPUS.log"
```

Read each raw `prime.json`, `warm-1.json`, `warm-2.json`, source/workload metadata,
probe JSONL and both saved caches. Successful exit is insufficient. Run, per corpus:

```sh
python3 scripts/dfg_compaction/read_acceptance.py \
  "$DFG_EVIDENCE/set-1/base-$CORPUS" "$DFG_EVIDENCE/set-1/candidate-$CORPUS" \
  "$DFG_EVIDENCE/set-1/base-$CORPUS.jsonl" "$DFG_EVIDENCE/set-1/candidate-$CORPUS.jsonl" \
  "$CORPUS" "$MINIMUM_BYTES" \
  --expect-receipts "$DFG_EVIDENCE/binary-receipts.json" \
  > "$DFG_EVIDENCE/set-1/$CORPUS-verdict.json"
cat "$DFG_EVIDENCE/set-1/$CORPUS-verdict.json"
```

Use `MINIMUM_BYTES=314572800` for Prometheus, `78643200` for slicing. Run the copied
summarizer on all four JSONLs; retain every raw file and checksum, not just verdicts.
The reader treats diagnostic census as a different namespace; diagnostic records
cannot repair acceptance failures. The reader ignores only `PRISM_PROBE_OUT` when
comparing cross-build environments, because each process writes its own evidence.

Admission precedes metric comparison. `--expect-receipts` consumes the controller's
existing `binary-receipts.json` (the `base.probe` and `candidate.probe` entries);
without the flag it defaults to `BASE_DIR/../../binary-receipts.json`, preserving
the unchanged `night-run.sh` positional invocation. Every prime and warm report
must carry nonempty build/version fields matching those receipts: source SHA
(including the recorded dirty suffix), build identity, grammar, allocator, dirty
mode and CPG version **120 base / 121 candidate**. Receipts also require matched
release/MCP profile, compiler, Cargo, allocator and dirty mode, no `dfg-oracle`,
and navigation sidecar version **66**. The reader hashes the retained
`bin/{base,candidate}/prism-mcp` bytes against their expected SHA256s and binds
the raw commands and `run.json` to those exact binaries. It reads the adjacent
`preflight-result.json` to bind package version and the complete reported build
identity to the controller's tiny preflight. Receipts and their checksum custody
remain controller evidence, not authentication supplied by the reader.

Corpus commits and clean dirty counts (`0`, never null), absolute paths, positive
workload request counts and nonempty SHA256s must be present and agree across
sides. Each report's `config.repo_root` and each process's probe metadata must
name that corpus. The retained workload bytes and request counts are checked
against `corpus.json`, the before inventory's workload hashes and its parsed
MANIFEST entry; the inventory must also carry the MANIFEST file hash. Every
available after inventory must equal the before inventory. Settings, including
future config fields and driver options, must match. Only the bound per-side
cache paths and `PRISM_PROBE_OUT` paths differ; each prime uses the driver's
intentional `settle=0`, and the two warms use the recorded settle setting.

Each process must have the relevant probe stages listed in
`scripts/dfg_compaction/evidence.py` (`PRIME_TIMERS` / `WARM_TIMERS`). Every
timer, including additional language-specific stages, needs a nonempty name and
finite, nonnegative `wall_ms`; `cpu_ms` must also be finite and nonnegative on
the supported macOS/Linux MCP hosts. An explicit null CPU counter is allowed
only on unsupported platforms. Missing or malformed timers make admission INVALID.

The driver retains no successful-prime stderr file. The verdict explicitly says
`stderr absent` in that case; it never claims that stderr was scanned. Cache-save
admission uses the prime's status and two miss flags, both unique nonempty cache
files in the post-prime inventory, the matching post-run inventory, and the raw
report fields and sibling `<side>-<corpus>.log`. Any retained save-error marker
in those fields, the run log or an available raw `*.stderr` makes the set INVALID.
Missing/empty run logs, missing caches or changed cache sizes are INVALID too.
This cannot recover warning text that the original driver discarded; it relies
on the retained positive cache evidence and subsequent true hits.

### Re-score tonight's set-1 without rerunning corpora

Use the updated checkout's reader, leaving the runner's copied tools and original
verdicts/checksums intact. `--rescore` reads the same raw layout and additionally
requires the final `after-inventory.json`; it performs no writes or measurements.
Write new verdicts outside the original evidence directory:

```sh
DFG_EVIDENCE="$HOME/prism-evidence/mcp-memory/dfg-compaction-2026-10-14"
RESCORE_OUT=$(mktemp -d "$HOME/prism-evidence/mcp-memory/dfg-rescore.XXXXXX")
for CORPUS in prometheus slicing; do
  case "$CORPUS" in
    prometheus) MINIMUM_BYTES=314572800 ;;
    slicing) MINIMUM_BYTES=78643200 ;;
  esac
  python3 scripts/dfg_compaction/read_acceptance.py \
    "$DFG_EVIDENCE/set-1/base-$CORPUS" "$DFG_EVIDENCE/set-1/candidate-$CORPUS" \
    "$DFG_EVIDENCE/set-1/base-$CORPUS.jsonl" "$DFG_EVIDENCE/set-1/candidate-$CORPUS.jsonl" \
    "$CORPUS" "$MINIMUM_BYTES" --rescore \
    --expect-receipts "$DFG_EVIDENCE/binary-receipts.json" \
    > "$RESCORE_OUT/$CORPUS-verdict.json"
  cat "$RESCORE_OUT/$CORPUS-verdict.json"
done
```

INVALID means the retained set did not establish admission; missing provenance
must not be filled in from assumptions or repaired by a diagnostic run. Preserve
new verdicts with the reader's source revision and receipt hashes. The reader's
exit code is 1 for INVALID; an admitted numerical FAIL remains in the JSON verdict
and does not by itself produce a nonzero exit code.

## Pass rule and additional controls

For each corpus, require `min(base heap)-max(candidate heap)` at least the minimum
above, using `after_index.heap_in_use_bytes` from two true hits per build. Report
`after_index - after_load_repo` heap separately. Load interval is the elapsed-ms
difference between those stages. With allowance `max(5% of base median, 100 ms)`,
first pass if candidate maximum is at most base minimum plus allowance. Otherwise
require each set's range/median at most 10%, then compare medians with that same
allowance. Excess spread is INCONCLUSIVE for load; a stable excess is FAIL.
Candidate prime `after_call_edges.peak_bytes` must not exceed base prime on either
corpus; even one extra byte is FAIL. Report RSS, physical footprint, compressed
bytes and lifetime peak separately. A material resident increase needs controller
investigation even if live heap passes. Missing flags, mismatched provenance,
nonfinite/missing required counters, failed cache save or census-enabled acceptance
runs are INVALID. Any admissible heap/load/peak FAIL takes precedence over noise
in another criterion. Correctness differences reject activation independently.

After timed runs, run paired `prism nav --cache-dir <separate-side-cache> dfg-stats
--repo "$CORPUS_PATH" --edges` sequentially for each corpus; require nonempty
valid JSONL and byte identity using `cmp`. Retain stdout/stderr and hashes.
Copy and run the prepared `dfgE-scratch/controller-query.sh` after binding
`DFG_EVIDENCE`, `CORPUS_PATH` and `CORPUS=prometheus`. It freezes a real edge
from the matched edge dump into one shared JSON diff/source/sink, includes every
tracked supported source file as an unchanged companion (so the DFG row scans
cover the corpus), primes a separate query cache per build, and runs both
binaries sequentially with `/usr/bin/time -p`, stores probe/query outputs and
byte-compares JSON. Also time parent/candidate `--algorithm chop,taint,provenance` on Prometheus with
the same frozen diff, valid source/sink, settings and cache mode. Record total
query time and the matched `query.forward_reachable` / `query.all_locations_at`
probe intervals. Row iteration has no per-row allocation; report any material
query cost rather than inferring it from the layout.

In separate fresh `diagnostic-census/` directories, unset
`PRISM_PROBE_NO_CENSUS` and run each build/corpus sequentially in the same load
mode. Report arena capacity/owned row heap once, both rank arrays, edge IDs,
binding keys/ID vector capacities, adjacency representatives/neighbors, labels,
exact labels and RD identity sets. Logical occurrence counts stay separately
named. Serialized sizes use the counting writer. Census accounting greater than
process live heap is INVALID. Census timing/peak has no acceptance vote.

At most one follow-up complete set, `set-2`, is allowed for INVALID/noisy results
and only if there is no already-admissible heap/load/peak FAIL. Reverse order
within each corpus (candidate then base), repeat both corpora, preserve both sets,
and never use a later set to erase a FAIL. At the cap hold PR E and report.

The controller completes this readout with raw metrics/verdicts, source/probe SHAs,
receipt hashes and quiet-host evidence, then commits
`docs: record matched DFG compaction acceptance` with both plan trailers.
Only Task 7 PASS and PR E merge authorize Task 8 oracle retirement. Full catalog
mutation and excluded socket gates remain controller pre-merge checks.

PR F handoff: `DFG-E-stable-neighbor-tie-sorted-by-byte` currently exercises
the temporary compact query engine (`compact/queries.rs`), not production
traversal. PR F retargets this mutant and its span-tie regression to production
`DataFlowGraph` traversal before retiring the temporary engine.
