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
python3 -m unittest discover -s scripts/memreport -p 'test_*.py'
```

The tests are split across several `test_memreport*.py` files to keep each under 600 lines; `discover`
runs all of them. One file alone: `python3 -m unittest scripts/memreport/test_memreport_rule.py`.

## What a run is

Four kinds per corpus, each repeated (default three times), one process at a time:

| Kind | Command adds | Measures |
|---|---|---|
| `warm` | `--cache-dir <run>/cache/<corpus>` after a priming run | load from the on-disk cache |
| `cold` | `--no-cache` | a full build, no cache written ("Cold peak (no cache write)") |
| `workload` | the corpus workload snapshot | steady state after queries and a settle ("Steady state (cache-loaded)") |
| `refresh` | `--mem-refresh` | the peak while two snapshots coexist |

The priming run's statistics are discarded, but its lifetime peak at its last stage is kept as "First-build
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

## What is excluded, and why

A run is inadmissible, and left out of every statistic, when any of these holds. Each means the run
measured something other than what its kind names. Excluded runs are listed with their reason in the
readout; they are never retried silently.

- it timed out or exited non-zero, or printed no single report line;
- it is not a version-3 report, or it is malformed: a field the driver reads has the wrong type (a `stages`
  entry that is not an object with a string `name`; a byte or count field that is not null or an integer from 0
  to 2^64-1, which rules out `1e309`, floats and booleans; a `build`, `env`, `cache` or `threads` of the wrong
  shape). The same check runs on every report, the priming run's included;
- a number its kind needs is unavailable, or its stage is missing (`warm`: `after_load_repo.peak_bytes`,
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
<run>/run.json                        settings: reps, settle, timeout, bin
<run>/workloads/<corpus>.jsonl        the snapshot every child read
<run>/raw/<corpus>/corpus.json        commit, dirty count, cache file sizes, workload digest
<run>/raw/<corpus>/prime.json         the priming run's result (its peak is the first-build peak)
<run>/raw/<corpus>/<kind>-<n>.json    one result per run: kind, rep, status, reason, report, host state
<run>/summary.json                    rewritten after each corpus finishes
```

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

## What the numbers are evidence of

- The decision rule (estimate at least 200 MiB or 15 % of the base, on at least two corpora) decides whether
  an experiment is worth running. M1 and M3 are judged against the steady-state footprint, M2 against the
  peak through the index load. It does not decide what ships; each candidate's experiment measures its real
  saving.
- The readout prints, per candidate and corpus, the estimate, the base, the percentage, which arm of the rule
  fired, the number of runs behind each, and a mark when that is below the requested repetitions.
- Freed memory can stay in the footprint, so nothing here is attributed by freeing and re-measuring.
