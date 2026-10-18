# Opt-in build profiling

Set `PRISM_PROFILE` to a writable JSONL path before starting `prism`. The file is
replaced for each process. An unset variable adds boundary checks only: no clocks,
resource sampling, owner metadata walks, or counter collection. An empty value
is an invalid path and warns once. Creation/write failures warn once on stderr
and preserve the command's output and status. Parent directories must exist.

```bash
PRISM_PROFILE=profile.jsonl prism nav --no-cache call-stats --repo tests/fixtures --dump-sites
PRISM_PROFILE=profile.jsonl prism nav --no-cache dfg-stats --repo tests/fixtures --edges
PRISM_PROFILE=profile.jsonl prism --repo small-repo --diff change.patch --algorithm leftflow
python3 scripts/perf/summarize_profile.py profile.jsonl
```

Use a different output path for concurrent processes. Profiles describe only work
performed by that command: an exact cache hit has no owner records; AST-only review
algorithms need no CPG. Phase records with `status: "not_run"` identify skipped
top-level work, including cache load/save with `--no-cache`. Forced process exits,
abort signals, and crashes cannot finalize a profile. The CLI owns a profiling
session; library callers can hold `prism::profile::Session::start()` until all their
build workers have joined. A session is process-wide and intended for one build
command, not simultaneous independently attributed builds.

## JSONL schema version 1

Every line is one JSON object. Record order is header, sorted phases, per-file RD
aggregates, owners sorted by file/start byte/name, summary. Field names and units
are stable; measured values and time ranking naturally vary. Additive fields may
be introduced without changing version 1. Timings are integer nanoseconds; memory
is bytes. Unknown platform measurements are JSON `null`.

| `record` | Fields / meaning |
| --- | --- |
| `header` | `schema_version`, `time_unit`, `memory_unit`, `peak_rss_scope`, `top_n` (10) |
| `phase` | `name`, optional `file`, `time_kind`, `aggregation`, `status`, `wall_ns`, `cpu_ns`, `peak_rss_bytes` |
| `owner` | `file`, `owner`, `start_line` (1-based), `start_byte` (0-based), `language`, `byte_size`, `ast_nodes`, `definitions`, `assignment_paths`, `total_ns`, `flow_edges_ns`, `flow_search_ns`, `reaching_definitions_ns`, `rd_cpu_ns`, `peak_rss_bytes`, `time_kind`, `counters` |
| `summary` | `time_kind: "summed_owner_work"`, `owner_count`, `total_owner_ns`, `top_owners`, `top_dfg_share`, `other_owners` |

`serial_wall` is a whole-phase interval enclosing any Rayon work. Its CPU is
process CPU, including all workers. `parallel_work` is an interval on a single
worker; its CPU uses the thread clock. Do not add parallel intervals to serial
wall time. `aggregation: "single_interval"` identifies measured intervals;
`"sum_owner_intervals"` identifies per-file RD sums. Inclusive parent phases
(`call_graph`, `cpg_assembly`, `dfg`) overlap their children. CPU clocks and RSS
return null on unsupported platforms. RSS comes from shared `getrusage`:
macOS `ru_maxrss` is bytes; Linux KiB is multiplied by 1024. It is the process's
lifetime peak sampled after an interval, not incremental or isolated phase memory.
For RD aggregates it is the maximum of samples after their constituent intervals.
MCP keeps using the same shared RSS helper with unchanged units and semantics.

The phase inventory includes `load_parse`, `scope_graph`, `call_graph` (prelude,
definitions, calls, merge, resolution facts, refresh, indirect), `dfg`, `dfg.file`,
`dfg.file_extract`, `dfg.file_rd`, `cpg_assembly` (init and Steps 1, 2–3, 4, 5, 5b, 5c, 6–9),
`cache.load`, and `cache.save`. Scope construction is nested within call-graph
construction. Skeleton and direct-subset constructors also emit `call_graph`,
with `call_graph.skeleton.*` and `call_graph.direct_subset.*` subphases.
`cpg.incremental` encloses partial-cache coordination: proof checks, removal,
direct CG/DFG construction, merge, refresh and assembly. Its separate
`call_graph.incremental_refresh` interval is outside the direct-subset constructor;
`call_graph` is a constructor interval, not an aggregate of every CG-labelled row.
A single command may therefore emit multiple records for a phase name. CPG Step 5c includes the subsequent refusal pass. Step 9 includes
final index ordering. Cache records cover the CPG cache transport, including miss
checks and post-load cleanup. They do not include lazy navigation query edge caches.

Owner identity follows the DFG producer, including synthetic anonymous owners;
language is the Rust language enum name. Byte size and AST node count include the
entire owner's subtree (including nested functions), like the search domain of
legacy named passes. Each actual DFG owner pass is counted once, even for owners
without definitions. `definitions` is RD input definitions, including aliases and
kill-only captured writes; `assignment_paths` counts admitted assignment spans,
including repeated assignments to the same path. `flow_edges_ns` includes owner
preparation and candidate construction before RD; `flow_search_ns` is the subset
spent in wrapped reference-search calls. RD includes lazy file CFG initialization
and applying RD labels. Total owner time includes profiling metadata collection.

Search counters count the DFG producer's `find_path_reference_spans_fenced_in`
calls, not output edges or unique names. Other extraction/projection walks remain
in `flow_edges_ns`.
`simple_searches` and `member_searches` distinguish access paths; `searched_bytes`
is the sum of requested AST-range sizes (a work proxy, not actual bytes read or
visited nodes). Parameters and alias-resolved paths may add searches beyond the
assignment count. Owner lines are never thresholded, so all owners contribute to
the summary. Normal-owner percentiles use nearest rank; empty populations have
count/total/percentiles/max zero. The top share denominator is summed owner DFG
work, excluding file setup/compaction and cross-file combination. It must not be
interpreted as a share of serial build wall time.

Inner `binding_fence_calls`, `shadow_check_calls`, and `def_scope_calls` are
currently `null` (uncollected). Completing them is pending the owner decision
about thin const-specialized hooks inside the search bodies shared with S1.
They must never be interpreted as zero. S0 is not complete until that decision
and its corresponding implementation or explicit scope change.
