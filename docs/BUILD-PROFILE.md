# Opt-in build profiling

Set `PRISM_PROFILE` to a writable JSONL path before starting `prism`. A complete file is
published through a unique sibling staging file and atomic rename. Concurrent writers
to one destination have last-completed-publication-wins semantics; files never mix.
A failed run preserves the previous destination. Aborts may leave a staging file,
never a truncated destination. An unset variable adds boundary checks only: no clocks,
resource sampling, owner metadata walks, or counter collection. An empty value
is an invalid path and warns once. Creation/write failures warn once on stderr
and preserve the command's output and status. Parent directories must exist.

```bash
PRISM_PROFILE=profile.jsonl prism nav --no-cache call-stats --repo tests/fixtures --dump-sites
PRISM_PROFILE=profile.jsonl prism nav --no-cache dfg-stats --repo tests/fixtures --edges
PRISM_PROFILE=profile.jsonl prism --repo small-repo --diff change.patch --algorithm leftflow
python3 scripts/perf/summarize_profile.py profile.jsonl
```

Profiles describe only work
performed by that command: an exact cache hit has no owner records; AST-only review
algorithms need no CPG. Phase records with `status: "not_run"` identify skipped
top-level work, including cache load/save with `--no-cache`. Normal explicit CLI exits finalize through `profile::exit` (including nonzero
outcomes); abort signals and crashes cannot finalize a profile. The CLI owns a profiling
session; library callers can hold `prism::profile::Session::start()` until all their
build workers have joined. A session is process-wide and intended for one build
command, not simultaneous independently attributed builds. Collection is active only
inside that session; a long-running library/MCP process without a session does not
collect or create a profile. Sequential sessions are allowed after workers join.

## JSONL schema version 1

Every line is one JSON object. Record order is header, command totals, sorted phases, per-file RD
aggregates, owners sorted by file/start byte/name, summary. Field names and units
are stable; measured values and time ranking naturally vary. Additive fields may
be introduced without changing version 1. Timings are integer nanoseconds; memory
is bytes. Unknown platform measurements are JSON `null`.

| `record` | Fields / meaning |
| --- | --- |
| `header` | `schema_version`, `time_unit`, `memory_unit`, `peak_rss_scope`, `top_n` (10), `prism_version`, `commit`, `thread_count`, `argv`, `os` |
| `command` | whole session `wall_ns`, process `cpu_ns`, `peak_rss_bytes`, `time_kind: "serial_wall"`; excludes final profile serialization/publication |
| `phase` | `name`, `instance` (occurrence ordinal), optional `file`, `time_kind`, `aggregation`, `status`, `wall_ns` (or `work_ns` for RD sums), `cpu_ns`, `peak_rss_bytes`, `rss_rise_bytes` |
| `owner` | `file`, `owner`, `start_line` (1-based), `start_byte` (0-based), `language`, `byte_size`, `ast_nodes`, `definitions`, `assignment_paths`, `total_ns`, `flow_edges_ns`, `flow_search_ns`, `reaching_definitions_ns`, `rd_cpu_ns`, `rd_rss_rise_bytes`, `file_cfg_init_ns`, `file_cfg_rss_rise_bytes`, `peak_rss_bytes`, `rss_rise_bytes`, `time_kind`, `counters` |
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
For RD aggregates it is the maximum of samples after their constituent owners.
`rss_rise_bytes` is the high-water-mark delta across the interval. Owner rises
subtract the lazy file-CFG interval's rise (`file_cfg_rss_rise_bytes`); RD sums aggregate
only `rd_rss_rise_bytes`, sampled from RD start to owner end, matching RD work/CPU. It is meaningful only at `RAYON_NUM_THREADS=1`;
parallel workers and
earlier allocations share the same high-water mark. A zero rise does not mean no
allocations. The renderer labels both columns "process peak RSS (high-water mark,
sampled after)".
MCP keeps using the same shared RSS helper with unchanged units and semantics.

The phase inventory includes `load_parse`, `scope_graph`, `call_graph` (prelude,
definitions, calls, merge, `js_ts_facts`, `parameter_slots`, `js_ts_props`, `populate_scope_graph`,
`js_anonymous_proofs`, `rust_macro_shadow_intersection`, `struct_init`,
`rust_receiver_refresh`, `go_dispatch_embedding`, `python_properties`,
`framework_entries`, `apply_js_paths`, `apply_js_export_resolution`, indirect), `dfg`, `dfg.file`,
`dfg.file_extract`, `dfg.file_cfg`, `dfg.file_rd`, `cpg_assembly` (init and Steps 1, 2–3, 4, 5, 5b, 5c, 6–9),
`cache.load`, and `cache.save`. Scope construction is nested within call-graph
construction, within `call_graph.populate_scope_graph`. The four initializer rows
measure imported-property proofs, scope construction, anonymous-callable proofs,
and the Rust transparent-macro shadow intersection, in that order. `struct_init`
covers moving the resulting values and empty/default fields into the graph. Skeleton and direct-subset constructors also emit `call_graph`,
with `call_graph.skeleton.*` and `call_graph.direct_subset.*` subphases.
`cpg.incremental` encloses partial-cache coordination: proof checks, removal,
direct CG/DFG construction, merge, refresh and assembly. Its separate
`call_graph.incremental_refresh` interval is outside the direct-subset constructor;
`call_graph` is a constructor interval, not an aggregate of every CG-labelled row.
A single command may therefore emit multiple records for a phase name. Each phase
has an additive `instance` ordinal (starting at 1 per `(name, file)` in the
command's record order, which preserves collection order within that pair).
The renderer shows `name #instance`; older v1 rows without the field still render.
These ordinals distinguish occurrences, not parent/child relationships or constructor types.
CPG Step 5c includes the subsequent refusal pass. Step 9 includes
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
spent in wrapped reference-search calls. Lazy file CFG initialization has its own per-file `dfg.file_cfg` parallel-work
interval, and `file_cfg_init_ns` on the owner that triggered it; both include building the CFG source-line edge index. It is excluded
from owner RD, total, flow-edge time, RSS rise and ranking. RD begins after CFG initialization
and includes applying RD labels. The owner clock starts after the profiling-only
AST node count and metadata walk.

Search counters count the DFG producer's `find_path_reference_spans_fenced_in`
calls, not output edges or unique names. Other extraction/projection walks remain
in `flow_edges_ns`.
`simple_searches` and `member_searches` distinguish access paths; `searched_bytes`
is the sum of requested AST-range sizes (a work proxy, not actual bytes read or
visited nodes). For simple searches, it measures requested scope size, not indexed candidate work.
Parameters and alias-resolved paths may add searches beyond the
assignment count. Owner lines are never thresholded, so all owners contribute to
the summary. Normal-owner percentiles use nearest rank; empty populations have
count/total/percentiles/max zero. The top share denominator is summed owner DFG
work, excluding file setup/compaction and cross-file combination. It must not be
interpreted as a share of serial build wall time.

Inner `binding_fence_calls`, `shadow_check_calls`, and `def_scope_calls` count
executed calls of `js_ts_reference_fenced`, `is_shadowed_at`, and `js_ts_def_scope`
during the current DFG owner pass, including refused gates and alias searches.
Binding fences include both reference-search fences and captured-write admission
fences against matching RD definitions; the latter may admit or refuse a write.
Short-circuit refusals do not count gates that never execute. Counters are owner-local;
const-specialized search hooks do no per-node counter work when disabled.
The manual no-query fallback in `scoped_refs` calls the recursive walker directly;
its shadow checks are not included in `shadow_check_calls`.

Command totals expose unprofiled work (hashing, registry, queries, algorithms and
normal output) as the remainder after non-overlapping top-level intervals. Do not
subtract every inclusive/nested phase or parallel sum from command wall time.
The command clock ends before profile rendering; normal CLI diagnostics emitted
by Rust after main returns are outside this guard.
