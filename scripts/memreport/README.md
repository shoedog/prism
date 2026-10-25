# memreport: where one prism-mcp index's memory goes

Measurement only. `prism-mcp` has a report mode (`PRISM_MCP_MEM_REPORT=1 … --eager`) that builds
the index in stages, optionally runs a query workload and one refresh, prints one JSON line to
stderr (`{"prism_mem_report":3,…}`) and exits without serving. This driver runs that mode per
corpus and renders a readout.

```bash
cargo build --release --bin prism-mcp --features mcp
python3 scripts/memreport/memreport.py gen-workload --bin target/release/prism-mcp --corpus ruff=~/code/bench-repos/ruff --workloads ~/prism-evidence/mcp-memory/c1/workloads
python3 scripts/memreport/memreport.py run --bin target/release/prism-mcp --corpus ruff=~/code/bench-repos/ruff --workloads ~/prism-evidence/mcp-memory/c1/workloads --out ~/prism-evidence/mcp-memory/c1/run-1
python3 scripts/memreport/memreport.py render --run ~/prism-evidence/mcp-memory/c1/run-1 --out docs/analysis/readout.md
python3 scripts/memreport/memreport.py compare --experiment m2 --baseline <run-a> --candidate <run-b> --out compare.md --json compare.json
python3 -m unittest discover -s scripts/memreport -p 'test_*.py'
```

The tests are split across several `test_memreport*.py` files to keep each under 600 lines; `discover`
runs all of them. One file alone: `python3 -m unittest scripts/memreport/test_memreport_rule.py`.

## What a run is

Four default kinds per corpus (`build` and `changed-refresh` on request), each repeated (default three times), one process at a
time:

| Kind | Command adds | Measures |
|---|---|---|
| `warm` | `--cache-dir <run>/cache/<corpus>` after a priming run | load from the on-disk cache |
| `cold` | `--no-cache` | a full build, no cache written ("Cold peak (no cache write)") |
| `workload` | the corpus workload snapshot | steady state after queries and a settle ("Steady state (cache-loaded)") |
| `refresh` | `--mem-refresh` | the peak while two snapshots coexist |
| `changed-refresh` (only with `--kinds`) | `--mem-changed-refresh <frozen edit.json>` | ordinary full refresh after an exact edit in a pinned clean private mirror; checked restoration |
| `build` (only with `--kinds`) | `--cache-dir <run>/cache-build/<corpus>/<rep>`, a new empty directory per repetition | a first build that writes the cache, repeated: `build_peak` (peak at `after_call_edges`) and `build_ms` (`after_index` minus `after_load_repo` elapsed) |

`--kinds LIST` (default `warm,cold,workload,refresh`) runs a comma-separated subset of
`warm,cold,workload,refresh,build,changed-refresh`, in that order whatever the order given. An unknown or repeated name, or an
empty list, is refused before anything runs. `warm` also reports `load_ms` (`after_index` minus
`after_load_repo` elapsed time; unavailable if either time is). The readout still lists the four kinds only
and prints `n/a` for those that were not run; `build` figures are read by `compare`.

A `build` repetition is admissible only if its report says `cache.cpg_hit: false` and
`cache.call_edges_loaded: false`, it passes the usual structural and build-identity checks and has the
`after_call_edges` peak, **and the process left both cache files behind**: after it exits the driver records the
sizes of `cpg-cache-v2.bin` (or `cpg-cache.bin`, which builds from before the rename wrote) and `resolved-call-edge-index.bin` (`cache_files` in the stored result), and both must be
integers above 0. The binary catches a failed cache save, prints it and still exits 0 with both flags false, so
the files are the evidence that the cache was written; stderr containing `failed to save CPG cache` or
`nav edge cache: failed to save sidecar` also excludes the run. `summarize` and `compare` re-apply the file check to
the stored sizes (a stored build result without `cache_files` is inadmissible).

Each `build` repetition gets its own cache directory, `<out>/cache-build/<corpus>/<rep>/`, and the driver
deletes it afterwards. Changed-refresh copies use the same ownership and cleanup mechanism. The driver never deletes by path: a path can be
made to name something else between any two looks at it. The directory is created exclusively, relative to an
open descriptor of its parent, and opened at once (a repetition is refused if the directory already exists, if
`cache-build` or the corpus directory is a symlink, or if what is found right after the creation is not
empty). That descriptor is kept until the measurement is over. The contents are then deleted through it,
wherever the directory is by then, and the name is removed with `rmdir`, which cannot remove a directory that
has content. A directory moved into the path during the measurement is therefore never touched. The stored
result says what was observed: `cache_dir_removed` is true only if the path is gone afterwards, and
`cache_dir_note` says why not otherwise (the path is also named on stderr, and `compare` prints a warning). Any
filesystem error while creating the directory (a file in the way, a read-only directory, a full disk) refuses
that repetition (inadmissible, no child run), not the whole run. The then-empty `cache-build/` directories are
removed. `build` keeps the settle sleep of `--settle` seconds per repetition; that is harmless, since the peak it
reports is a lifetime peak.

A corpus NAME (in `--corpus NAME=PATH`, for `run` and `gen-workload`) must be letters, digits, `.`, `_` or `-`,
starting with a letter or digit: it names directories under `--out`.

The priming run happens only when at least one of `warm`, `workload`, `refresh`, `changed-refresh` is selected. Its statistics are discarded, but its lifetime peak at its last stage is kept as "First-build
peak (writes the cache, n=1)": one build that also writes the cache, shown as `n/a` when the priming run is
absent or was not admitted. Right after it the driver measures the size of `cpg-cache-v2.bin` (and of
`resolved-call-edge-index.bin`) under `<run>/cache/<corpus>/`; a file that is absent, or not the only one of
that name, is recorded as unavailable, not as 0.

## Changed refresh (opt-in support seam)

Build with `--features memreport-changed-refresh` and select `--kinds changed-refresh`, supplying
`--changed-edit NAME=PATH` for each corpus. Use a dedicated private mirror. The manifest contains exactly
`repo_root` (canonical absolute path), `commit`, `path` (relative regular file without symlinks), `before`,
`after` (distinct nonempty UTF-8 content), `before_sha256`, and `after_sha256`.
The driver freezes the manifest and verifies the actual starting bytes before any child, then again before
each changed repetition. Each repetition copies the pristine prime into its own exclusively created
`cache-changed/<corpus>/<rep>` directory, and removes that copy after success, failure or interruption.
Recovery uses only the original bytes verified from the target; a failed run stops subsequent repetitions.
Rust checks clean pinned identity again before editing and checks restoration even if refresh unwinds.
Observed before/after/restored hashes are retained in the report. Abort/SIGKILL of the parent cannot run cleanup;
inspect the mirror before using it again.

Set `PRISM_SUPPORT_PATCH_SHA256=<support patch digest>` **on both baseline and candidate sides**. Reported
`PRISM_*` environment values bind the common support seam and are checked by comparison. Defaults stay unchanged.
The FullOnly S0 validator requires strategy `full` and generation 1; later strategy prototypes must update it
explicitly. Admission here checks primary peaks; complete auxiliary timing/heap/footprint/RSS/tree availability
must be checked by downstream optimization acceptance gates.

Manual mutant execution is documented in `mutants/manual/README.md`; feature/Python mutants are deliberately
outside the shared runner's `mutants/*.json` catalog.

## What is measured and what is estimated

| Figure | Kind | Source |
|---|---|---|
| Stage footprints and peaks | measured | the process's own footprint and peak counters |
| Steady live heap, index live heap | measured | bytes the allocator reports as in use (macOS: `size_in_use` over all zones; glibc: `uordblks + hblkhd`). On glibc, blocks freed into a thread cache stay counted and allocator bookkeeping is included, so it can sit above the live allocations; it needs glibc 2.33 or later (`mallinfo2`, found at run time) and is `n/a` elsewhere |
| **M1** retained trees | measured | live tree-sitter heap after the workload and settle (`tree_sitter_heap_bytes`): every live tree-sitter allocation, so the retained trees plus prism's compiled query cache (about 54 KB, fixed per process). An upper bound on what dropping the trees frees; trees that are re-parsed later would come back |
| **M2** cache buffer | measured file size | size of `cpg-cache-v2.bin`: the buffer `fs::read` allocates and streaming would remove, so an upper bound on the peak saving |
| Observed load overshoot | measured, lower bound | peak above the larger of the load peak and the built footprint; freed memory stays in the footprint, so 0 is uninformative |
| **M3** repeated strings, "attributed" | estimate | computed from component counts, not from allocation measurement |

A figure that is unavailable (the sampler had no source, a stage is missing) is `n/a` in the readout and
absent from every statistic; it is never 0.

**"Steady state (cache-loaded)" is not a repeatable figure on macOS.** After a cache load the footprint was
seen at one of two levels, hundreds of MiB apart, with the same live heap at both, and a longer `--settle`
did not decide which (measured in `docs/analysis/2026-10-prism-mcp-memory-readout.md`, last section; the
mechanism was not identified). "Steady live heap" is repeatable, but it counts allocations, not physical
memory. A baseline-against-candidate comparison of steady state therefore uses the live heap with a
pinned minimum effect size, and a candidate whose saving is itself live heap (dropping the retained trees)
must also show it in footprints compared between runs at the same level; footprint minus live heap tells the
levels apart (spec §6.1 as revised 2026-10-13).

## What this tool does not defend against

Raw results are trusted as written. `summarize` and `compare` refuse files that are misnamed, duplicated or
out of range, which catches accidents (a copied file, a stray backup). They do not authenticate a measurement:
someone who edits a result file by hand, or copies one under a valid name with its fields changed, can change
a verdict. The evidence directory's checksums, taken when a run finishes, are what detect that.

## What is excluded, and why

A run is inadmissible, and left out of every statistic, when any of these holds. Each means the run
measured something other than what its kind names. Excluded runs are listed with their reason in the
readout; they are never retried silently.

- it timed out or exited non-zero, or printed no single report line;
- it is not a version-3 report, or it is malformed: a field the driver reads has the wrong type (a `stages`
  entry that is not an object with a string `name`; a byte or count field that is not null or an integer from 0
  to 2^64-1, which rules out `1e309`, floats and booleans; a `build`, `env`, `cache` or `threads` of the wrong
  shape). The same check runs on every report, the priming run's included;
- a number its kind needs is unavailable, or its stage is missing (`build`: `after_call_edges.peak_bytes`; `warm`: `after_load_repo.peak_bytes`,
  `after_index.peak_bytes`, the `after_index` footprint; `cold`: `after_call_edges.peak_bytes`; `refresh`:
  `after_refresh.peak_bytes`; `workload`: the `after_settle` footprint);
- a warm, workload or refresh run whose report does not say `cache.cpg_hit: true` (the CPG cache was missed or
  partly missed) or does not say `cache.call_edges_loaded: true` (the call-edge index was computed instead of
  loaded from its sidecar: the sidecar was absent, undecodable, from another build or tree, or refused on a
  dirty build). The process records this itself; a run is never admitted merely because it printed no
  complaint, since an absent or undecodable sidecar prints nothing. Such runs read about 7 % higher at steady
  state. Stderr text from the cache code (`CPG cache miss`, `partial CPG cache hit`, `nav edge cache: skipping
  warm sidecar load`, `nav edge cache: failed to load sidecar`) also excludes a run and names the cause;
- a run whose thread count, measured-process environment, allocator or platform (os, arch) differs from the first
  admitted run of its corpus (the field is named);
- a run whose build identity differs from the first admitted run of its corpus, or a corpus whose build
  identity differs from the first corpus's (by name) that has an admitted run; the corpus is excluded whole;
- a `build` run whose report does not say that both caches missed (`cache.cpg_hit` and
  `cache.call_edges_loaded` must be `false`, not `true` and not `null`): it was not a first build; or that wrote
  no cache file (above);
- a refresh run did not report `status: refreshed` (`raced_stale` means the tree changed during it);
- a workload run had errors, answered fewer requests than it sent, ran none, or ran a workload whose request
  count or sha256 differs from the one the driver gave it.

A binary built from a working tree with uncommitted changes skips the call-edge sidecar, so **every warm,
workload and refresh run of such a binary is inadmissible** unless `PRISM_NAV_EDGE_CACHE_LOAD_DIRTY=1` is
set in the driver's environment (children inherit it). The readout's Provenance section prints
`binary_input_dirty`, the rayon thread count and every `PRISM_*` variable the measured process saw, so a run made with the override
is visibly one.

## Workloads

`workloads/<corpus>.jsonl` is a fixed list of `tools/call` requests generated once per corpus commit;
`workloads/MANIFEST.json` records the commit, request count and sha256. A baseline and a candidate build
must run the same file. Regenerate only when the corpus commit changes.

- `--workloads DIR` (on `gen-workload` and `run`) keeps them somewhere else. Use it when the corpus is this
  repository, otherwise the generated files make the corpus dirty.
- `gen-workload` writes the manifest after each corpus. A corpus that times out, fails or emits nothing is
  reported and skipped; the command exits non-zero naming the failed corpora.
- `run` refuses a corpus that is not a git checkout with a commit (git cannot give a commit, so the workload
  cannot be bound to the tree); the readout prints `n/a` for a commit or dirty count it does not have.
- `run` reads each workload once and refuses to start unless its sha256 and request count match the manifest
  and the manifest's commit is the corpus's current commit (re-run `gen-workload` otherwise). It copies those
  bytes to `<run>/workloads/<corpus>.jsonl` and gives that snapshot to every child. Each workload child
  reports the sha256 of the bytes it read; a mismatch makes that run inadmissible. The digest and count are
  recorded in `summary.json`.

## Run directory

```
<run>/run.json                        settings: reps, settle, timeout, bin, kinds
<run>/workloads/<corpus>.jsonl        the snapshot every child read
<run>/raw/<corpus>/corpus.json        commit, dirty count, cache file sizes, workload digest
<run>/raw/<corpus>/prime.json         the priming run's result (its peak is the first-build peak)
<run>/raw/<corpus>/<kind>-<n>.json    one result per run: kind, rep, status, reason, report, host state
                                      (swap, compressor, load average)
<run>/cache/<corpus>/                 the primed cache the warm, workload and refresh runs read
<run>/cache-build/<corpus>/<rep>/     a build run's own empty cache directory; deleted after its run
<run>/summary.json                    rewritten after each corpus finishes
```

`raw/<corpus>/build-<n>.json` also holds `cache_files` (the sizes of the cache files that run wrote) and
`cache_dir_removed`. `kinds`
in `run.json` is the list that ran; `summarize` does not need it (it scores whatever is in `raw/`), so a
`run.json` from before `--kinds` existed (no `kinds` key, meaning the default four) still summarizes, as does a
directory that holds only some kinds.

A run directory holds one run: `run` refuses to start when `<run>/raw`, `run.json` or `summary.json`
exists (use a new `--out`, or `summarize` to rescore), so results of two runs are never pooled.

## Rescoring saved evidence: `summarize`

```bash
python3 scripts/memreport/memreport.py summarize --run <run>
```

rebuilds `summary.json` from `raw/` alone, running every stored report through the current admissibility
rules again, the cache evidence in each report (`cache`) and the build-identity checks included. A rule fix,
or a new rule, is applied to evidence that took hours to collect without running anything. It can only
exclude more: stderr markers are seen while a run happens and are not stored for admitted runs, so a marker
cannot be re-applied (the report's own `cache` section can). `render` then rebuilds the readout from
`summary.json`.

Rescoring fails closed on a directory it cannot trust: every `*.json` in a corpus directory other than
`corpus.json` and `prime.json` must be named `<kind>-<rep>.json` for the kind and repetition it holds, with
`rep` an integer from 1 to the run's `reps`; a copy (`warm-1 copy.json`), a result filed under another name or a
repetition outside that range makes `summarize` and `compare` exit with a message naming the file, because it
would otherwise count as another repetition. A `run.json`, `corpus.json`, `prime.json` or result that is not a
JSON object, or a directory without `run.json` or `raw/`, is refused with a message too.

## Comparing two runs: `compare`

```bash
python3 scripts/memreport/memreport.py compare --experiment m2 --baseline <run-a> --candidate <run-b> \
    --out compare.md [--json compare.json] [--allow-same-build]
```

rebuilds both summaries in memory from each directory's `run.json` and `raw/` with today's rules (the code path
`summarize` uses; a saved `summary.json` is never read, and a directory without `raw/` is refused), runs nothing,
and writes a Markdown report: both runs' provenance (build identity, git sha, CPG cache version, allocator,
`binary_input_dirty`, platform, threads, environment, repetitions, settle, the host's 1-minute load average as
median and maximum over the admitted runs, `n/a` for runs recorded before it was kept, and the per-corpus commit),
bold warnings (a binary built from a dirty tree; a corpus with uncommitted files, since equal counts are not equal
trees; a build that left its cache directory behind; one repository given under two corpus names), the rule, one table per metric (per corpus: baseline and candidate median with min-max and n, the
difference of the medians in absolute terms and as a percentage, the outcome), a verdict table with one line per
metric, and an **Excluded runs** table per side (corpus, kind, repetition, reason; "None." when empty). Run
directories are named by their last path component (parent/last when the two are the same). `--json` writes the same content as data, the rule's
constants and text included. Bytes are MiB with one decimal, durations whole milliseconds, an unavailable value
`n/a`.

It **refuses** (non-zero exit, every difference named, nothing written) unless the two runs have the same
corpus names; per corpus the same commit, dirty count and workload sha256, and (where both sides have an admitted
run) the same thread count, measured-process environment, allocator and platform, which must be known; the same
repetitions and settle; and **different** build identities (the same build is refused unless `--allow-same-build`
is given, for a noise check; the report then says so). One directory against itself is always refused.

Experiment `m2` needs `--kinds warm,workload,build` on both runs: `peak` and `load_ms` come from `warm`,
`steady_heap` from `workload`, `build_peak` and `build_ms` from `build`. A metric no run measured is "not
measured".

The pinned rule of experiment `m2` (a table in `memreport_compare_rule.py`, `EXPERIMENTS`, so another experiment is
a new entry; a gate metric must be in bytes):

| Metric key | Label | Unit | Role |
|---|---|---|---|
| `peak` | Load peak (cache-loaded, through the index load) | bytes | gate |
| `build_peak` | First-build peak (writes the cache) | bytes | gate |
| `load_ms` | Index load time | ms | not worse |
| `build_ms` | Index build time | ms | not worse |
| `steady_heap` | Steady live heap | bytes | not higher |

- **gate**, per corpus: *separated* = candidate max < baseline min; *reduction* = baseline median - candidate
  median; *effect* = reduction of at least 100 MiB or at least 5 % of the baseline median. The corpus **passes**
  when both series have at least 3 runs and it is separated and has an effect. With fewer than 3 runs on either
  side it is "insufficient runs": not a pass and not a fail. The metric **passes** when at least 2 corpora pass.
  Otherwise it is **undecided** when fewer than 2 corpora exist, or when the corpora that pass or could still pass
  with more data (pass, insufficient runs, one-sided, not measured) number at least 2; else **fail**.
- **not worse**, per corpus, with r = candidate median / baseline median (always shown, e.g. `×1.031`; the JSON
  row has `ratio`, `above_band` and `ranges_apart`): **unresolved** when either side has fewer than 3 runs; else
  **ok** when the candidate median is at most 1.05 times the baseline median; else **worse** when the ranges do
  not overlap (candidate min > baseline max); else **unresolved** (above the band, ranges overlap). A difference
  inside the overlap of the two ranges is not evidence either way: on this machine a second run of the same build
  differed by 15 % in load time while other work was running. The metric is **worse** if any corpus is, else
  **unresolved** if any is, else **incomplete** if any is one-sided, else **ok**; its Verdict line shows the
  largest ratio.
- **not higher**, per corpus: ok when the candidate median is at most 1 MiB above the baseline median. A lower
  figure is ok: a candidate that allocates more tightly lowers it without changing the index.
- A series on exactly one side is **one-sided** for that corpus; on neither side, **not measured**. For not higher
  the metric is **higher** if any corpus is, else **incomplete** when any corpus is one-sided, else ok; a metric absent from both runs everywhere is "not measured" and decides nothing. The
  boundaries are exact (a candidate median of exactly 1.05 times is ok, a live heap exactly 1 MiB higher is ok, a
  candidate max equal to the baseline min is not separated).

The comparison decides nothing about what ships: it reports the numbers and the pinned rule's outcome.

## What the numbers are evidence of

- The decision rule (estimate at least 200 MiB or 15 % of the base, on at least two corpora) decides whether
  an experiment is worth running. M1 and M3 are judged against the steady-state footprint, M2 against the
  peak through the index load. It does not decide what ships; each candidate's experiment measures its real
  saving.
- The readout prints, per candidate and corpus, the estimate, the base, the percentage, which arm of the rule
  fired, the number of runs behind each, and a mark when that is below the requested repetitions.
- Freed memory can stay in the footprint, so nothing here is attributed by freeing and re-measuring.

## Idle lifecycle collection (I0c-2)

`idle` is a separate normal-serving path. Existing `run` stays eager and unchanged.
I0 remains off by default; these commands explicitly request a policy and never activate it globally.
Python 3.9+, standard library only. Live ledger/clock sampling requires macOS; other hosts retain UNKNOWN
readings. The portable fixture/injection tests run there; only the live Apple ABI test skips.

```sh
python3 scripts/memreport/memreport.py idle collect --manifest "$E/arm.json" --out "$E/run-A-1"
python3 scripts/memreport/memreport.py idle readout --run "$E/run-A-1/run.json" \
  --run "$E/resident-for-A-1/run.json" --out "$E/readout.json"
# Once, on a separate unscored instance/window, never during latency or relief:
python3 scripts/memreport/memreport.py idle cross-check --pid PID --out "$E/cross-check"
```

Each collect invocation launches one fresh stdio process or one shared shim and its fresh daemon.
Shared uses `--shared --index-idle-secs N [--index-idle-pressure-relief]`, requiring the parallel Rust
forwarding slice. It requires a private, empty `runtime_dir` base per run, never the default runtime.
The shim stderr and daemon key-log are both tailed. STATUS corroborates PID/start/key/settings and client
count; more than the single launched shim excludes the run because membership is incomplete.
The daemon's READY/STATUS epoch seconds join event `instance_start` after multiplication by 1e9.
Each process is additionally bound to kernel OS start identity at discovery and every sweep.
The driver closes stdin/reaps its client and sends instance-bound daemon SHUTDOWN on every exit path.
It records unconfirmed shutdown; the operator must resolve that before another run.

A preregistered manifest (replace paths, pins and hashes with actual frozen values):

```json
{
  "binary": "/absolute/path/prism-mcp", "binary_sha256": "SHA256",
  "commit": "COMMIT", "features": ["mcp"],
  "repo": "/private/mirror", "cache_dir": "/private/cache",
  "corpus": "py", "corpus_pin": "COMMIT", "workload_hash": "SHA256",
  "cache_class": "warm", "mode": "stdio", "arm": "A", "pair_id": "A-rep-1",
  "requested_idle_secs": 600, "requested_relief": false,
  "startup": "lazy", "first_call_wait_secs": 20, "cycles": 2, "score_cycle": 1,
  "first_request": {"name": "nav_callees", "arguments": {"seed": {
    "kind": "symbol", "name": "run", "file": "main.py"}}},
  "edges_unused": false, "max_polls": 3, "poll_interval_secs": 0.1,
  "prerequisites": {
    "soak_complete": true, "c0_pins": true, "free_bytes": 53687091200,
    "quiet_serial": true, "swap_non_increasing": true, "reserved_window": true,
    "all_prerequisites_landed": true
  }
}
```

Preregister these prerequisites with the controller; booleans record the supplied receipts, not independent
host/soak verification. The binary is hashed locally; a missing or differing declared hash excludes it.
Use corpus names `py`, `e`, `p`, `ruff` for the four-corpus readout. Pair IDs identify independent repetitions,
not cycles. Run at least three fresh instances per side. Every A/A+C/reference generation ends in unload;
`reference` defaults to one fresh-lazy cycle, using the reload arm's settings/cache and identical first request.
A+C requires relief; A uses the matched no-op. Scored enabled runs require 600 seconds. Optional Background
startup uses its launch as the original pre-warm demand; a pre-index sweep after build start is excluded.
No startup is delayed to manufacture a pre-index sample. Eager is refused on this path.

Collect X before its resident match. For each X=A or A+C, launch a **separate** resident run with idle=0,
relief=false, cycles=1 and `resident_pair` pointing to X's `run.json`; use the same pair ID, pins and binary.
Before launch, the collector requires one unique preregistered scored candidate row. It records X's
arm/instance/cycle/attempt/G and response-to-disposal delta on the resident row and verifies them when scoring.
Resident deadlines use its last response arrival + (X disposal_end - X last response arrival) +5/+30 seconds.
The default resident run budget is that delta +30 seconds +180 seconds of bounded launch/query overhead.
An explicit `timeout_secs` below that budget produces a failed run record before any launch.
Never pool resident samples across different X deadlines or interpolate. Score the preregistered cycle
(default initial cycle 1); subsequent cycles remain raw reload observations, not independent repetitions.
Polls replay only the preregistered first request, up to `max_polls`, and never restart its demand clock.
`prewarm: true` labels a preregistered demand; no bridge pre-warm implementation is provided here.

Raw output must be outside the checkout. `manifest.json`, `run.json`, `stdout.jsonl`, `stderr.log`, shared
key-log, `status.json`, `visibility.json` and `clocks.json` retain provenance, file identity/offset, every event,
request/response arrival, poll, failure and abandonment. Rows include cycle/attempt/generation, effective
settings, PID/start/key/OS start/client count, cache/sidecar facts, stages, readiness/build/first-served times,
sample due/start/end/lateness, member footprint/RSS, relief return/heap pair, and admission reasons.
JSON `null` means UNKNOWN; an absent reading is never zero. Raw failed/abandoned rows remain in readout.
`readout` rebuilds rows from raw facts instead of trusting a saved scored status. It compares known binary
hashes against each other and the preregistered pin; a failed launch's unknown hash remains a failed row.
For a short **unscored** parser smoke only, set `smoke_only: true`, idle=2, `smoke_hold_secs: 0` and a short
`timeout_secs`. It still waits for unload/reload but omits +5/+30; every row remains EXCLUDED.

The sampler uses the complete 296-byte Apple `rusage_info_v4` ABI via `proc_pid_rusage` flavor 4, checks
return/errno before fields, rejects exited/reused PIDs, sums every surviving shim/daemon once, and makes
an incomplete or changing inventory UNKNOWN. `ri_resident_size` is report-only RSS, never footprint.
Pre-index precedes both the first call and build; loaded is publication+query; idle deadlines use production
`disposal_end`, not log delivery. Sweeps start no earlier than due and finish within 250 ms, including all
members. Loaded waits for its linked served query to complete and retains the publication deadline; completion
after the 250 ms bound leaves loaded UNKNOWN. Late boundaries get no catch-up sampling. Reload/refresh/disposal-failure crossings exclude them.
Reads/deadline joins bracket realtime/monotonic within 1 ms; bad observations are re-read. Consecutive valid
observations enforce 10 ms +50 ppm offset continuity, while Mach continuous/absolute clocks detect >10 ms
sleep steps. Missing boundary clocks or sleep exclude, never bridge; cumulative drift remains report-only.
The CLI cross-check retains per-PID auxiliary bytes/times/differences and failures; it never substitutes a CLI
series or subtracts assumed overhead. A permission failure leaves the comparison UNKNOWN for the controller.

The parser accepts real stdio/daemon producer grammar, ignoring unrelated diagnostics. Sequence gaps,
duplicates, missing/malformed fields, wrong/restarted instances, missing/replaced/truncated streams and cap
notices exclude acceptance. After stripping transport prefix, newline and the leading-space metadata sequence
token, a UTF-8 body **>=967 bytes** is possibly capped. Hardlinks are deduplicated by inode/offset; repeated
sequence numbers at different offsets are errors. Event-scoped unload sidecar joins only the same
instance/attempt/generation publication. Readout uses `mono_ns`/`epoch_ns`, never cross-mode `mono_ms`.

M uses matched current idle physical footprint: median savings >=200 MiB **or** >=15% on >=2 corpora,
>=3 independent runs/side, candidate max < base min. The +30 s cap is <=128 MiB on admitted rows in all four corpora in both
modes. Any admitted over-cap row fails; any cell with no admitted row is UNKNOWN. Excluded and failed
rows retain their reasons and counts. A report-only boundary failure leaves its value UNKNOWN and never
filters M, the cap or reload latency; a later failed cycle does not taint an earlier scored cycle. 15%-of-base idle is report-only. Relief/no-op heap delta must stay within +/-1 MiB; missing heap
excludes that comparison. Exact-hit reload needs actual CPG hit in reload/reference, identical first request,
and ending unload sidecars hit/hit or legitimately unused not_attempted/not_attempted. Its median first-served
latency must be <= reference +max(5%,100 ms); demand-to-publication <=3 s is the separate readiness target.
The report keeps warming, served-within-wait, warming/poll/success, failure/abandonment counts and proportions
pooled and by corpus, mode, reload class and their arm-specific intersections. Reload CPG hit is the exact-hit
class; preregister `changed_snapshot_cycles` for changed-snapshot reloads supplied by a separate lane.
Other misses and unavailable facts have separate classes; a cache miss alone does not prove a changed snapshot.
This collector does not edit the workload between cycles.
RSS, pre-index deltas (+16 MiB reference), loaded/+5, idle fraction and optional relief footprint are report-only.
`memory_gate` is keyed by arm, with M and cap per mode and an arm summary requiring both modes.
A and A+C are independent activation alternatives. `relief` reports the +/-1 MiB heap comparison separately
per arm and mode (including A's no-op); it never filters physical M pairs. Missing heap excludes only that
relief comparison. `reload_latency` is separate. Activation always remains an owner
decision per arm after complete prerequisites, both modes, unit parity and admitted external evidence.

Tests: `python3 -m unittest discover -s scripts/memreport -p 'test_*.py'` and the soak no-regression suite.
Compressed fixtures in `test_memreport_idle.py` are byte-real ae49d937 stdio/direct-daemon captures (local filesystem paths in non-lifecycle diagnostic lines replaced by placeholders); edge tests
transform those bytes explicitly. Raw capture locations, hashes, REDs, GREENs, mutations and exclusions live
in the lane's external `VERIFICATION.md`. Additional modules split clock binding, scheduling, row joins,
normal-serving transport and CLI orchestration along those responsibilities; each stays below 300 weighted lines.
