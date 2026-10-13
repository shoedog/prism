# memreport: where one prism-mcp index's memory goes

Measurement only. `prism-mcp` has a report mode (`PRISM_MCP_MEM_REPORT=1 … --eager`) that builds
the index in stages, optionally runs a query workload and one refresh, prints one JSON line to
stderr and exits without serving. This driver runs that mode per corpus and renders a readout.

```bash
cargo build --release --bin prism-mcp --features mcp
python3 scripts/memreport/memreport.py gen-workload --bin target/release/prism-mcp --corpus ruff=~/code/bench-repos/ruff
python3 scripts/memreport/memreport.py run --bin target/release/prism-mcp --corpus ruff=~/code/bench-repos/ruff --out ~/prism-evidence/mcp-memory/c1/run-1
python3 scripts/memreport/memreport.py render --run ~/prism-evidence/mcp-memory/c1/run-1 --out docs/analysis/readout.md
python3 -m unittest scripts/memreport/test_memreport.py
```

## What a run is

Four kinds per corpus, each repeated (default three times), one process at a time:

| Kind | Command adds | Measures |
|---|---|---|
| `warm` | `--cache-dir <run>/cache/<corpus>` after a priming run | load from the on-disk cache |
| `cold` | `--no-cache` | a full build |
| `workload` | the corpus workload | steady state after queries and a settle |
| `refresh` | `--mem-refresh` | the peak while two snapshots coexist |

## What is excluded, and why

A run is inadmissible, and left out of every statistic, when it timed out, exited non-zero, printed
no single report line, missed the cache when it was meant to be warm, or had workload errors. Each
of those means the run measured something other than what its kind names. Excluded runs are listed
in the readout; they are never retried silently.

## What the numbers are evidence of

- Stage footprints and peaks are measurements of this process on this host.
- Tree and string byte figures are **estimates** from exact counts. They rank candidates.
- The decision rule (200 MiB or 15 % on two corpora) decides whether an experiment is worth
  running. It does not decide what ships; each candidate's experiment measures its real saving.
- Freed memory can stay in the footprint, so nothing here is attributed by freeing and re-measuring.

## Workloads

`workloads/<corpus>.jsonl` is a fixed list of `tools/call` requests generated once per corpus
commit; `workloads/MANIFEST.json` records the commit, request count and hash. A baseline and a
candidate build must run the same file. Regenerate only when the corpus commit changes.
