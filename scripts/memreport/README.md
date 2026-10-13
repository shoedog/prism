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

Four kinds per corpus (a fifth, `build`, on request), each repeated (default three times), one process at a
time:

| Kind | Command adds | Measures |
|---|---|---|
| `warm` | `--cache-dir <run>/cache/<corpus>` after a priming run | load from the on-disk cache |
| `cold` | `--no-cache` | a full build, no cache written ("Cold peak (no cache write)") |
| `workload` | the corpus workload snapshot | steady state after queries and a settle ("Steady state (cache-loaded)") |
| `refresh` | `--mem-refresh` | the peak while two snapshots coexist |
| `build` (only with `--kinds`) | `--cache-dir <run>/cache-build/<corpus>/<rep>`, a new empty directory per repetition | a first build that writes the cache, repeated: `build_peak` (peak at `after_call_edges`) and `build_ms` (`after_index` minus `after_load_repo` elapsed) |

`--kinds LIST` (default `warm,cold,workload,refresh`) runs a comma-separated subset of
`warm,cold,workload,refresh,build`, in that order whatever the order given. An unknown or repeated name, or an
empty list, is refused before anything runs. `warm` also reports `load_ms` (`after_index` minus
`after_load_repo` elapsed time; unavailable if either time is). The readout still lists the four kinds only
and prints `n/a` for those that were not run; `build` figures are read by `compare`.

A `build` repetition is admissible only if its report says `cache.cpg_hit: false` and
`cache.call_edges_loaded: false`, and passes the usual structural and build-identity checks and has the
`after_call_edges` peak. After the process exits the driver records the sizes of the cache files it wrote
(`cache_files` in the stored result) and deletes that repetition's cache directory (hundreds of MiB each).

The priming run happens only when at least one of `warm`, `workload`, `refresh` is selected. Its statistics are discarded, but its lifetime peak at its last stage is kept as "First-build
peak (writes the cache, n=1)": one build that also writes the cache, shown as `n/a` when the priming run is
absent or was not admitted. Right after it the driver measures the size of `cpg-cache.bin` (and of
`resolved-call-edge-index.bin`) under `<run>/cache/<corpus>/`; a file that is absent, or not the only one of
that name, is recorded as unavailable, not as 0.

## What is measured and what is estimated

| Figure | Kind | Source |
|---|---|---|
| Stage footprints and peaks | measured | the process's own footprint and peak counters |
| Steady live heap, index live heap | measured | bytes the allocator reports as in use (macOS: `size_in_use` over all zones; glibc: `uordblks + hblkhd`). On glibc, blocks freed into a thread cache stay counted and allocator bookkeeping is included, so it can sit above the live allocations; it needs glibc 2.33 or later (`mallinfo2`, found at run time) and is `n/a` elsewhere |
| **M1** retained trees | measured | live tree-sitter heap after the workload and settle (`tree_sitter_heap_bytes`): every live tree-sitter allocation, so the retained trees plus prism's compiled query cache (about 54 KB, fixed per process). An upper bound on what dropping the trees frees; trees that are re-parsed later would come back |
| **M2** cache buffer | measured file size | size of `cpg-cache.bin`: the buffer `fs::read` allocates and streaming would remove, so an upper bound on the peak saving |
| Observed load overshoot | measured, lower bound | peak above the larger of the load peak and the built footprint; freed memory stays in the footprint, so 0 is uninformative |
| **M3** repeated strings, "attributed" | estimate | computed from component counts, not from allocation measurement |

A figure that is unavailable (the sampler had no source, a stage is missing) is `n/a` in the readout and
absent from every statistic; it is never 0.

**"Steady state (cache-loaded)" is not a repeatable figure on macOS.** After a cache load the footprint was
seen at one of two levels, hundreds of MiB apart, with the same live heap at both, and a longer `--settle`
did not decide which (measured in `docs/analysis/2026-10-prism-mcp-memory-readout.md`, last section; the
mechanism was not identified). "Steady live heap" is repeatable, but it counts allocations, not physical
memory. Which figure a baseline-against-candidate comparison should use is an open owner question (lane
README, question 5); until it is answered, compare footprints only between runs at the same level
(footprint minus live heap tells the levels apart).

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
- a run whose build identity differs from the first admitted run of its corpus, or a corpus whose build
  identity differs from the first corpus's (by name) that has an admitted run; the corpus is excluded whole;
- a `build` run whose report does not say that both caches missed (`cache.cpg_hit` and
  `cache.call_edges_loaded` must be `false`, not `true` and not `null`): it was not a first build;
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
<run>/cache/<corpus>/                 the primed cache the warm, workload and refresh runs read
<run>/cache-build/<corpus>/<rep>/     a build run's own empty cache directory; deleted after its run
<run>/summary.json                    rewritten after each corpus finishes
```

`raw/<corpus>/build-<n>.json` also holds `cache_files`, the sizes of the cache files that run wrote. `kinds`
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

## Comparing two runs: `compare`

```bash
python3 scripts/memreport/memreport.py compare --experiment m2 --baseline <run-a> --candidate <run-b> \
    --out compare.md [--json compare.json] [--allow-same-build]
```

reads the two `summary.json` files (it runs nothing) and writes a Markdown report: both runs' provenance (build
identity, git sha, CPG cache version, allocator, threads, environment, repetitions, settle, per-corpus commit),
the rule, one table per metric (per corpus: baseline and candidate median with min-max and n, the difference of
the medians in absolute terms and as a percentage, the outcome) and a verdict table with one line per metric.
`--json` writes the same content as data. Bytes are MiB with one decimal, durations whole milliseconds, an
unavailable value `n/a`.

It **refuses** (non-zero exit, every difference named, nothing written) unless the two runs have the same
corpus names; per corpus the same commit, dirty count and workload sha256; the same repetitions and settle; the
same thread count, measured-process environment and allocator; and **different** build identities (the same
build is refused unless `--allow-same-build` is given, for a noise check; the report then says so).

The pinned rule of experiment `m2` (a table in `memreport_compare.py`, `EXPERIMENTS`, so another experiment is a
new entry):

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
  side it is "insufficient runs": not a pass and not a fail. The metric passes when at least 2 corpora pass; if
  it does not, it is "insufficient runs" when corpora short of runs could still bring the passes to 2, else
  "fail".
- **not worse**, per corpus: ok when the candidate median is at most 1.05 times the baseline median. The metric
  is ok when it is ok on every corpus that has both series; a corpus with a missing series is listed as "not
  measured".
- **not higher**, per corpus: ok when the candidate median is at most 1 MiB above the baseline median. A lower
  figure is ok: a candidate that allocates more tightly lowers it without changing the index.
- A metric absent from both runs is "not measured" and decides nothing. The boundaries are exact (a candidate
  median of exactly 1.05 times is ok, a live heap exactly 1 MiB higher is ok, a candidate max equal to the
  baseline min is not separated).

The comparison decides nothing about what ships: it reports the numbers and the pinned rule's outcome.

## What the numbers are evidence of

- The decision rule (estimate at least 200 MiB or 15 % of the base, on at least two corpora) decides whether
  an experiment is worth running. M1 and M3 are judged against the steady-state footprint, M2 against the
  peak through the index load. It does not decide what ships; each candidate's experiment measures its real
  saving.
- The readout prints, per candidate and corpus, the estimate, the base, the percentage, which arm of the rule
  fired, the number of runs behind each, and a mark when that is below the requested repetitions.
- Freed memory can stay in the footprint, so nothing here is attributed by freeing and re-measuring.
