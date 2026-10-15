# DFG Compaction Implementation Plan

> **For agentic workers:** Execute task by task using superpowers:executing-plans. Implementer: **gpt-6.1-sol, Codex, xhigh**. Controller dispatches **Claude Opus 5.5 + gpt-6-astra** for each PR, with a **two-round review cap** declared before dispatch. Steps use checkboxes. This document is the execution brief; no conversation history is required. This plan authorizes no implementation in the planning session.

**Goal:** Reduce each resident Prism index enough to avoid swap exhaustion during multi-agent work: at least **300 MiB less live heap after a cache hit on Prometheus**, at least **75 MiB less on slicing**, no measured cold-build peak increase, and bounded cache-load latency (Task 7).

**Architecture:** Store every byte-distinct `VarLocation` once in an immutable, graph-owned arena; reference it with checked `u32` occurrence IDs. Maintain a separate ordered legacy-identity index, plus an occurrence representative in each adjacency/label row. Edges and neighbor vectors retain their original sequence and multiplicity. Keep complete `VarLocation`/`AccessPath` values at the arena boundary so queries can borrow them; do not introduce a process-global interner. Decode the new cache directly into this representation. Compact each file's temporary producer result before collecting the parallel build, then combine compact parts; updates filter/reindex compact records without materializing an expanded whole graph.

**Tech Stack:** Rust 2021; existing `BTreeMap`, `BTreeSet`, `Vec`, serde/bincode 1, Rayon, tempfile, Python 3; no new dependency or unsafe code.

**Spec/evidence:** Owner's 2026-10-14 request; `.review/FINDINGS.md:3-7,23-43`; `.review/probe-summary-prometheus.md:137-151`; `.review/probe-summary-slicing.md:95-109,146-160`; `.review/astra-optimization-analysis.md:18-20,41-53,113-125` and §4 item 4; `scratch-opt-probe` commit `c3162f94`, `src/probe/census.rs:3-13,301-390`. House style: `docs/superpowers/plans/2026-10-12-shared-prism-daemon/S3b-concurrent-dispatch.md`. Source baseline inspected here: **`8b2744b3154b3664ffacb23c42f042a67981967a`**, branch `plan/dfg-compact`. Rebind source anchors if main moves; do not transplant changed semantics into the frozen oracle.

**Validation status:** This is a source-grounded plan, not a compiled production patch. A standalone Rust model using extracted source DTOs/lattice passed **10 tests**, including the ordering/representative counterexamples below. It did not link Prism or test serde, fixtures, incremental construction, output renderers, or the cache. No cargo command, full suite, Tier-A, mutation gate, or real-corpus process ran during planning. Full command/output provenance is in “Planning validation” below. Do not repeat S3b's much stronger validation claim for this plan.

## Global Constraints

- Preserve observable output **byte for byte**: navigation, review, SARIF, targets, MCP Evidence; preserve node/edge insertion order, first-wins representatives, stable sorting, ties, confidence and telemetry. Do not bless new goldens. Cache bytes intentionally change only when its version changes.
- Use `BTreeMap`/`BTreeSet` and explicitly ordered iteration. Numeric occurrence IDs are **not legacy sort keys**. Never substitute a HashMap or unstable sort in a behavior-bearing path.
- Keep `VarLocation::{Eq,Ord,Hash}`, `ExactVarLocation`, `AccessPath`, `FlowConfidence::worst`, RD classification, CPG endpoint selection and query algorithms semantically unchanged.
- Each new Rust/Python file is under **600 lines**, and split modified source/test modules by category before they exceed 600 lines. Existing oversized callers require mechanical extraction, as detailed in the file map and Task 2; do not bury semantic changes in those moves. The plan document and frozen evidence captures are not source modules.
- Run `cargo fmt`, **full** `cargo test` and **full** `cargo test --features mcp` at each PR gate. Count passed/failed/ignored tests across every target and doc test; filtered focused runs do not replace these suites. Record out-of-scope failures with a same-environment base control, not a new baseline.
- Every added behavior has a watched RED (missing-API compile RED is only scaffold evidence), a behavioral mutant or pre-change failure, and a negative/edge test. Behavior-neutral moves use byte controls and deliberately wrong mutants; do not invent a regression that the old behavior should fail.
- Mutation gate each review round: `python3 scripts/mutgate/mutgate.py --since main --scope fn --jobs 1`; before merge: `python3 scripts/mutgate/mutgate.py --authoritative --jobs 1` with **no `--since` or `--only`**. Register compaction mutants; the old catalog alone will not test new code. Scoped schema observations are advisory. A zero-mutant selection is not coverage.
- Any change to call resolution, navigation, CPG construction (`src/call_graph.rs`, `src/navigation/`, `src/cpg/`, `src/ast.rs`, or type providers feeding them) runs a fresh release build immediately before Tier-A matrix, before commit; Tier-A `--quick` before review. Run both for every PR in this lane, including codec-only PRs, because compatibility is its purpose. Use `--allow-stale-sut` only after that worktree's rebuild. Paste regressions/flip candidates into PR descriptions; never re-baseline. `INVALID` is not green. Full multi-corpus Tier-A is human-triggered.
- A serialized-layout change bumps `src/cpg_cache.rs`'s `CACHE_VERSION` (currently **117**, source line 261). Navigation sidecar stays **65** (`src/navigation/call_edge_cache.rs:110`) unless its serialized layout/semantics change; there is no intended sidecar change. Reserve **118** only if still free at execution; otherwise use current main + 1 and update version tests.
- Host: **one cargo process at a time**, always `nice -n 19 env CARGO_BUILD_JOBS=4 cargo ... > logfile 2>&1`, then read the logfile separately; never pipe cargo into grep/tail. No installs/fetches or browser launches in this sandbox. If priority setting or a required command is refused, route to controller; do not drop the restriction and run a heavy build anyway.
- Mutgate invokes cargo itself and defaults to four workers (`scripts/mutgate/mutgate.py:339-350,654`). Use `--jobs 1` and the cargo wrapper below, inherited through PATH, to enforce the same host rule. Controller reserves the build slot for the entire gate, not individual subprocesses.
- Never `git stash`, `git commit --amend`, reset, or normalize unrelated work. The controller commits in a host context where `.git` writes are allowed. Every task has a commit step; do not try sandbox commits. Trailers on every implementation commit: `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>` and `Claude-Session: https://claude.ai/code/session_01JhYB2xRuHRHxDFM7eDt8Fh`.
- Review discipline: WRONG requires input/state, wrong result, bounded fix and fail-first regression; report WRONG before SMELL. Two rounds per PR. At the cap classify: converging closed findings get a disclosed targeted extension; recurring open-class defects stop for design correction. No fresh restart of a partially reviewed artifact without owner authority.
- Before a debugging probe record prediction, falsifier and alternative cause; afterward record which occurred. Probe/schema/environment failures are inadmissible. Attribution to this change needs the base in the same environment.
- Controller runs real-corpus measurements, sequentially, one `prism-mcp` at a time. Implementer prepares binaries and receipts. Do not run Prometheus/slicing during implementation or this planning task.
- At stable points snapshot work and refresh the lane handoff using `/Users/wesleyjinks/.claude/handoff-template.md` (canonical steering source `bootstrap/handoff-template.md`). Do not alter memory without an explicit owner request. Leave commit/push to controller.

## Decisions

| ID | Decision | Reason and cost if wrong |
|---|---|---|
| D1 | Intern **complete exact locations**, not their legacy Eq class. `OccId` identifies all fields including bytes. `LegacyId` identifies the old Eq/Ord tuple. | Same-line references would otherwise lose bytes. Cost if wrong: CPG endpoint selection/Evidence changes; stop release. |
| D2 | Arena sorted by **ExactVarLocation's** field order; a second index sorts by **VarLocation's** order. | Exact order places bytes before kind; legacy order excludes bytes and orders kind. A single ordinal cannot serve both. Cost: extra two u32 arrays, about 1.7 MiB for the measured Prometheus core population. |
| D3 | Per-map representative IDs, separate from identity IDs; label values store both original endpoint IDs. | BTree entry retains its first key. Forward, backward, labels and each vector may retain different spans. Cost: 4 bytes per adjacency row and 8 per label row, already included in sizing caveats. |
| D4 | `edges` and neighbor sequences are vectors; do not deduplicate or globally sort them. | `remove_files` and `merge` rebuild adjacency from the retained/concatenated edge sequence. Stable sort/dedup occurs only where the old query does it. Cost if wrong: tie selection and byte output drift. |
| D5 | Complete `VarLocation`s own their `String`s and `AccessPath`s in the arena. Keep binding-map keys `(String,String,usize,AccessPath)` initially. **No separate string/path pool in this lane.** | This still interns every repeated exact location and its entire owned path/string payload. It preserves `&VarLocation` access without hydration caches. The measured core unique string/path heap is only 11.38 MiB Prometheus / 11.06 MiB slicing (not the whole DFG). Cost if wrong: less saving for defs-only populations; Task 7 rejects activation if the minimum is missed. A second pool is a separately measured follow-up, not an unbounded contingency here. |
| D6 | No simultaneously resident production legacy+compact graphs; dual representation only in tests. | Keeping an oracle beside the production graph would defeat the lane goal. Cost: tests use more memory; execute corpus cases sequentially. |
| D7 | Expose ordered accessor views; retain query signatures. Direct public-field consumers migrate. IDs remain private. | Keeping mutable legacy public fields prevents invariant maintenance. Cost: source compatibility for external direct-field library users; those fields are internal under `src/api/mod.rs:1`, while the stable facade/handle contracts remain unchanged. Use the repository-wide accessor migration with a release note. |
| D8 | Native compact cache and runtime activation ship **atomically** after a test-only codec PR. | Old-format decode followed by whole-graph conversion raises peak and load cost. Cost: one invalidation/cold rebuild, no old-cache importer. |
| D9 | Build compact per-file records before collecting parallel producer results; updates remap compact records and reclaim dead arena rows. | Never compact by building another full legacy graph or retaining dead-string tombstones across refreshes. Cost: O(live records) remap at update boundaries; measure refresh separately as diagnostic, not a claimed improvement. |
| D10 | Bounded two-run acceptance uses conservative range separation, not statistical significance. At most one complete follow-up set. | Historical slicing load time varied 3.2/1.8 s on a busy host. Two samples cannot justify a broad performance claim. Cost: a noisy host can yield INCONCLUSIVE; no threshold relaxation after seeing candidate data. |

## Evidence and derived sizing

**Measured (inherited, not rerun):** Prometheus warm DFG 964.2 MiB, process live heap 2,295.3 MiB; slicing warm DFG 364.2 MiB, live heap 1,549.1–1,549.2 MiB (summary citations in header). BTree node overhead/allocator rounding are excluded. Census string quantities are **bytes**: `.review/FINDINGS.md:35` loosely calls 143,047,946 bytes “143 MiB”; the exact summary/census fields mean **136.42 MiB**, not 143 MiB. Use raw bytes for arithmetic.

| Component, inherited accounted MiB | Prometheus | slicing | Proposed disposition |
|---|---:|---:|---|
| edges | 318.5 | 80.8 | two u32 occurrence IDs per edge, same order |
| definitions | 28.0 | 23.6 | binding keys retained; vector members become IDs |
| uses | 176.1 | 116.4 | binding keys retained; vector members become IDs |
| forward | 126.7 | 38.5 | legacy ID key + representative occurrence ID + neighbor IDs |
| backward | 146.3 | 51.6 | same |
| labels | 168.4 | 53.1 | legacy ID pair + representative occurrence pair + unchanged confidence |
| exact labels, RD stats | 0.0, 0.1 | 0.0, 0.2 | exact ID pairs; RD stats unchanged; zero is rounded, not absence |

Source: Prometheus summary `:149-151`; slicing `:107-109`. Serialized totals **551.3 / 216.9 MiB** are bincode size proxies, not live heap.

**Derived, not a whole-graph prediction:** the census core population is only `edges.from/to + forward keys/values + backward keys/values` (`c3162f94:src/probe/census.rs:337-372`). With `n` occurrences, `u` exact distinct locations, occurrence heap `h`, and legacy-distinct heap `hu`, estimate old core `136*n+h`, compact core `136*u+hu+4*n+8*u`. The `8*u` conservatively charges both lookup arrays; `hu` slightly understates exact heap because 16/40 extra exact rows were not separately heap-counted. This also omits new representative/row headers and BTree slack. Do not call it a guaranteed floor or apply 12.3× to all DFG fields.

| Corpus | n | exact u | old core MiB | estimated compact core MiB | derived difference |
|---|---:|---:|---:|---:|---:|
| Prometheus | 2,766,125 | 224,497 | 495.19 | 52.76 | 442.42 MiB |
| slicing | 723,395 | 175,779 | 138.42 | 37.96 | 100.46 MiB |

`u` differs from legacy identity counts 224,481 / 175,739 and `(file,line,path)` counts 197,274 / 166,561. Those coarser counts are not legal interning keys. Layout 136 bytes is inherited from `.review/astra-optimization-analysis.md:41-53` and reported by the census. Arithmetic was rerun during planning (Planning validation). **Guessed whole-DFG saving:** 400–650 MiB Prometheus, 90–180 MiB slicing; not an acceptance result. The preregistered 300/75 MiB thresholds deliberately leave room for omitted overhead and extra exact/defs-only rows. Resident footprint and swap savings must be reported separately from live heap.

## PR split and measurement stops

| PR | Tasks | Independently reviewable result | Runtime heap effect | Stop before next |
|---|---|---|---|---|
| A — oracle | 1 | Frozen legacy implementation, exact-byte query/corpus oracle; production unchanged | derived 0 | Full oracle on baseline; no corpus process |
| B — accessor boundary | 2 | Ordered read API and mechanical callers/module extraction; old storage still live | derived 0 | Both full suites + Tier-A + mutation |
| C — compact engine | 3–4 | Test-only arena, ID maps, query parity, remove/merge and collection kernel | derived 0 in production; guessed candidate 400–650 / 90–180 MiB | Measure compact allocation accounting on the fixture/generator corpus before codec work; retain the record. Controller real-corpus acceptance follows activation, before cleanup. |
| D — compact codec | 5 | Test-only validated wire codec, corruption/version/stream parity tests; production format unchanged | derived 0 | Full corpus round trips and malformed IDs rejected |
| E — activation | 6–7 | Compact producer, runtime and cache v118 together; acceptance readout | guessed 400–650 / 90–180 MiB; **must measure ≥300/75** | Task 7 passes before cleanup PR starts; no production legacy fallback |
| F — remove migration scaffolding | 8 | Remove old production layout/temporary activation switch; frozen oracle remains test-only | derived 0 if E truly had no legacy storage; any measured saving here signals a failed E invariant | Final acceptance pair + authoritative gate before merge |

Review sizing targets are **guessed budgets**, not measured patch sizes: A/C/D/F ≤800 semantic lines each; B ≤500 semantic lines plus separately marked mechanical moves; E ≤600 semantic lines plus producer extraction. Each individual source file remains under 600. If a PR exceeds its semantic budget, split at a named task boundary **before dispatch**, never after rejected review. Mechanical extraction may be more lines; reviewers receive a byte/method-body comparison and review semantic hunks separately. No full graph rewrite in one PR.

## Review Focus

1. Equal legacy locations with different bytes, and Def/Use at conflicting byte order → Task 3 `legacy_identity_excludes_bytes_exact_identity_does_not`, `kind_precedes_bytes_only_in_legacy_order`.
2. A label map inserted in a different order from the edges; forward/backward key representatives differ; duplicate neighbors and stable dedup → Task 3 `map_representatives_are_independent`, `adjacency_retains_duplicates_and_input_order`, `equal_legacy_neighbors_stable_sort_keeps_first_span`.
3. Overlapping merges with `Killed(7)` versus `Killed(3)`, non-edge labels, empty binding rows, cross-file synthetic edges → Task 4 `overlap_merge_and_remove_match_legacy_bytes`, `removal_uses_map_key_file_and_either_edge_endpoint`.
4. Unknown exact spans in a valid legacy query, absent locations, cycles, same-line assignment propagation → Task 1/3 corpus oracle and Task 3 `unknown_exact_query_can_still_find_legacy_key`; query implementation must not reject a legacy lookup because exact lookup failed.
5. Exact supplemental labels with same-line duplicates, UTF-8 spans and confidence conflict refusal → Tasks 1/5 corpus + existing `src/cpg/exact_caller_read_tests.rs` and `same_line_occurrence_tests.rs`; compare insertion-order CPG bytes and Evidence, not just Eq.
6. Cache corruption whose ID is in range but belongs to the wrong legacy group, malformed sort/duplicates, stale version, partial file, borrowed-vs-owned serializer → Task 5 corruption matrix and `compact_stream_and_owned_bytes_match`.
7. Repeated remove/re-add and old snapshot held through clone → Task 4 `twenty_refreshes_reclaim_dead_locations_and_leave_clone_unchanged`; no reused external IDs, dead rows or leaked strings.
8. Cold producer temporarily retaining all expanded per-file results, or warm decode expanding the old layout → Task 6 build-mode assertion and Task 7 cold prime peak; a warm-only heap win cannot waive cold-peak failure.

## Consumer inventory and migration

Planning searches: `rg -l '\bDataFlowGraph\b|\.dfg\b' src tests`; `rg -n '\bdfg\b|\bdata_flow\b|DataFlowGraph' src/algorithms src/navigation src/cpg src/cpg_cache.rs src/cpg_cache src/call_graph.rs`; `rg -n 'data_flow|dfg_' src/reasoning src/algorithms/taint.rs`. These are structural inventories, not a semantic call-graph claim. Repeat on execution base and record new hits before editing.

| Consumers | Current use / migration |
|---|---|
| `src/data_flow.rs` | All storage, constructors, `build_subset`, `remove_files`, `merge`, query methods. Extract types, build helpers and queries. Queries use views with the same FIFO/stable-sort/first-insert semantics. |
| `src/cpg/build.rs:89-99,623-686,1026-1188` | RD stats, subset/prune/merge, defs/uses materialization, legacy edges/labels and exact supplemental edges. Use `defs()/uses()/edges()/label()/exact_labels()`; RD stats accessor/mutator; retain assembly order and refusal logic byte for byte. |
| `src/cpg/query.rs`, `cfg_queries.rs`, `trace.rs`, `reaching.rs`, `reaching/{capture,scope,tests}.rs` | Public DTOs and CPG-backed data-flow queries. DTOs unchanged; no query rewrite. Compare graph assembly/query output against oracle-built CPG. |
| `src/algorithms/{full_flow,left_flow}.rs` | `!dfg.edges.is_empty()` → `!dfg.edges().is_empty()` via view. |
| `src/algorithms/delta_slice.rs:40-68` | `dfg.edges.iter()` → `dfg.edges().iter()`; line-key set construction unchanged. |
| `src/algorithms/provenance_slice.rs:548,627` | `backward_reachable_labeled`/`backward_reachable` signatures unchanged. Their exact EvidenceHop bytes are oracle outputs. |
| `src/algorithms/taint.rs`, `src/reasoning/taint_reaches.rs`, finding-confidence/Evidence renderers | DTO and CPG-query consumers, no direct DFG storage dependency. Keep call sites and serialization untouched; run algorithm/reasoning/output/MCP suites and dual CPG output controls. |
| `src/navigation/mod.rs:158`, `src/cpg_cache.rs:311,452,495,630,752`, `src/cpg_cache/stream.rs:96-105` | Clone for refresh; owned and borrowed serde; full hit and incremental hit. Clone compact records; native validated codec at activation. No lazy expanded cache. |
| `src/call_graph.rs:9924-10175`, `src/cpg/tests.rs`, `src/ast_{asserted_member,required_parameter}_tests.rs`, `src/executable_owner/integration/tests.rs` | Empty/build constructors or field assertions in tests. Constructors unchanged; observation assertions use accessors, synthetic mutation uses test-only legacy-parts builder. |
| `src/cpg/{exact_caller_read,inert_default_parameter,namespace_flow_audit,nested_execution_owner_audit,optional_parameter,same_line_occurrence}_tests.rs` | Field iteration, exact-label mutation, bincode round trip, clone/subset/incremental. Migrate to views and `test_support::edit_legacy_parts` for deliberate inconsistent synthetic state; do not weaken existing exact-byte assertions. |
| `tests/ast/{access_path,binding,cpg_cache,dfg_label,dfg,field}_test.rs`, `tests/common/mod.rs` | Direct maps/edges, corpus generators, cache. Adapt assertions to view snapshots; all generators included below. |
| `tests/integration/{cjs_export_state,cjs_refusal_state,cjs_terminal_state,core,dfg_label_parity,dfg_label_store,dfg_rd_stats_identity,esm_forwarding_state,js_binding_export_state,js_binding_namespace,js_paths,js_wrapped_export_state,module_binding_flow,receiver_self_binding,resolution,type_relative_receiver}_test.rs`, `tests/integration/js_binding_namespace/spec_r2.rs` | Build/clone/storage observations; mechanical accessor changes only. Module binding/receiver algorithms unchanged. |
| `tests/algo/taxonomy/{misc,taint_lang,taint_sink}_test.rs`; `tests/lang/c/{algo,cve}_test.rs`; `tests/lang/go/{lang,loader_hygiene,owner_partition_telemetry,receiver_owner_carrying}_test.rs`; `tests/lang/javascript/{destructuring,lang}_test.rs`; `tests/lang/lua/lua_test.rs`; `tests/lang/rust/rust_test.rs`; `tests/lang/typescript/erased_rvalue_test.rs`; `tests/name_resolution/build_wiring_test.rs` | Constructors and field/query assertions; unchanged public answers, mechanical views. |
| Scratch `src/probe/census.rs`, `src/probe/census/sizer.rs` on `c3162f94` | Port physical-storage accounting to arena/IDs; never expand legacy rows to count candidate heap. Emit semantic occurrence counts separately. Scratch instrumentation is never merged. |

A `rg` hit on a comment or DTO is not authority to change that module. Conversely, aliases such as `cached_dfg`, `graph` and `file_refs` require the member-use search in Task 2; searching only `.dfg` is insufficient.

## File map

| Files | Responsibility / owning task |
|---|---|
| `src/data_flow.rs` | ≤200-line facade/re-exports; initially legacy storage, finally `DataFlowGraph { storage: CompactDfg }` |
| `src/data_flow/{types,views,queries,legacy_storage}.rs` | unchanged DTOs; ordered views; query bodies; transitional old fields (Tasks 1–2, removed from production in 8) |
| `src/data_flow/build/{mod,file,definitions,uses,labels,alias}.rs` | extract `build_from_refs` by file pass, def collection, use collection, classification and alias helpers (2,6); preserve text/order |
| `src/data_flow/compact/{mod,arena,freeze,views,update,wire,validate}.rs` | types; arena ranks; legacy-parts ingestion; views; compact prune/merge/reindex; codec; validation (3–5) |
| `src/data_flow/oracle/{mod,build,file,definitions,uses,labels,alias,queries,update}.rs` | **frozen** legacy implementation under `cfg(test)` or explicit `dfg-oracle` test feature; no production dependency (1) |
| `src/data_flow/tests/{identity,representatives,queries,updates,wire,corruption,corpus,observe}.rs` | split tests by category, each <600 lines (1,3–5) |
| `tests/dfg_compaction.rs`, `tests/dfg_compaction/{mod,corpus,outputs}.rs` | integration oracle using common generators, public CPG/output parity (1–8) |
| `src/cpg/dfg_assembly.rs`, `src/cpg/dfg_assembly_tests.rs` | mechanically extracted DFG materialization block; view migration here (2) |
| `src/cpg_cache/dfg_codec_tests.rs`, `src/cpg_cache/stream_tests.rs` | codec version/stream integration (5–6) |
| `scripts/dfg_compaction/compare_acceptance.py` | preregistered numeric readout and receipt checking (7), <600 lines |
| `mutants/dfg-compaction.json` | register the explicit mutants listed per task; retain lane conventions |
| `docs/superpowers/handoffs/2026-10-14-dfg-compaction.md` | living implementation handoff (controller snapshots/commits) |

Large existing modules touched by mechanical extraction are split along their existing function/test-module boundaries until each resulting source module is under 600 lines; retain re-exports and function visibility. In particular `src/cpg/build.rs`, `src/cpg_cache.rs`, oversized CPG test modules and `tests/common/mod.rs` cannot receive additional inline implementations. Put new code/tests in the named child modules. Submit extraction-only commits before behavior changes; if the necessary extractions cannot fit PR B's review budget, divide B into B1 production and B2 tests, each with its own complete gate. This is a declared optional PR boundary, not permission for a new design.

## Command protocol (applies to every task)

Create log directories with ordinary filesystem operations. Commands below run at the clone root unless explicitly in a subshell. Every `cat` is an output-reading step; Expected: the complete captured log, including failures. Do not execute a later dependent step after a failed command.

```bash
mkdir -p .review/dfg-compaction/logs .review/dfg-compaction/bin
```
Expected: directories exist; no source or git state changed.

For mutgate only, create `.review/dfg-compaction/bin/cargo` with this exact content, substituting nothing in the script (it locates the real cargo via rustup):

```sh
#!/bin/sh
exec nice -n 19 env CARGO_BUILD_JOBS=4 "$DFG_REAL_CARGO" "$@"
```

```bash
chmod +x .review/dfg-compaction/bin/cargo
export DFG_REAL_CARGO="$(rustup which cargo)"
export DFG_GATE_PATH="$PWD/.review/dfg-compaction/bin:$PATH"
```
Expected: executable wrapper; `DFG_REAL_CARGO` is an absolute real toolchain binary, not the wrapper. Controller confirms exclusive cargo slot and priority capability before any build. Do not set PATH globally for normal commands.

**GATE** means run all the following sequentially at that task/PR boundary, with its task number in the log directory. Record actual totals; there is no invented expected suite count:

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 cargo fmt --check > .review/dfg-compaction/logs/fmt.log 2>&1
cat .review/dfg-compaction/logs/fmt.log
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test > .review/dfg-compaction/logs/default.log 2>&1
cat .review/dfg-compaction/logs/default.log
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp > .review/dfg-compaction/logs/mcp.log 2>&1
cat .review/dfg-compaction/logs/mcp.log
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp,dfg-oracle --test dfg_compaction > .review/dfg-compaction/logs/oracle.log 2>&1
cat .review/dfg-compaction/logs/oracle.log
nice -n 19 env CARGO_BUILD_JOBS=4 cargo build --release > .review/dfg-compaction/logs/release.log 2>&1
cat .review/dfg-compaction/logs/release.log
(cd eval && uv run tier-a --matrix-only --allow-stale-sut) > .review/dfg-compaction/logs/tier-matrix.log 2>&1
cat .review/dfg-compaction/logs/tier-matrix.log
(cd eval && uv run tier-a --quick --allow-stale-sut) > .review/dfg-compaction/logs/tier-quick.log 2>&1
cat .review/dfg-compaction/logs/tier-quick.log
PATH="$DFG_GATE_PATH" python3 scripts/mutgate/mutgate.py --since main --scope fn --jobs 1 > .review/dfg-compaction/logs/mutgate-scoped.log 2>&1
cat .review/dfg-compaction/logs/mutgate-scoped.log
```
Expected, respectively: formatting clean; default suite all green with totals; mcp suite all green with totals; nonzero dual-oracle cases and zero differences; release binary rebuilt here; valid matrix without regressions; valid quick without regressions; selected registered mutants killed and an **advisory** scoped report. Preserve each task's logs before reuse. If `fmt --check` fails, run the same wrapped `cargo fmt`, read its log, then check again. If host facilities prevent Tier-A/any suite, record exact exclusions and controller route; do not claim the PR ready for merge.

Before every PR merge:

```bash
PATH="$DFG_GATE_PATH" python3 scripts/mutgate/mutgate.py --authoritative --jobs 1 > .review/dfg-compaction/logs/mutgate-full.log 2>&1
cat .review/dfg-compaction/logs/mutgate-full.log
```
Expected: authoritative **full** text-mode verdict passes, all selected survivors/timeouts/build errors disclosed. No simultaneous cargo process.

**Commit protocol:** controller stages only that task's file-map paths after the gates and a reviewed `git diff --check`. Each task's last step names its literal commit title; the exact commands and trailers are collected in the commit appendix. Expected: one new commit containing exactly that task, both trailers present; refresh the handoff and snapshot receipts. No commits are made by the planning worker.

---

### Task 1 (PR A): Freeze the independent legacy oracle and corpus inputs

**Files:** create oracle and test modules in the file map, `tests/dfg_compaction.rs`; add `dfg-oracle = []` to Cargo features. Production behavior and cache version stay unchanged.

**Interfaces:** frozen `LegacyDataFlowGraph` has the fields and public methods of baseline `DataFlowGraph`; it shares only the unchanged DTO/lattice types. Expose it as `data_flow::oracle::LegacyDataFlowGraph` only under `#[cfg(any(test, feature = "dfg-oracle"))]`. Production/default/mcp binaries do not enable `dfg-oracle`.

- [ ] **RED — Freeze source before editing it.** Copy the exact baseline `DataFlowGraph` declaration and complete implementation (`src/data_flow.rs:189-1519`) into the oracle modules, rename only the graph type, keep `VarLocation`, `FlowEdge`, `ExactFlowEdge`, `AccessPath`, RD APIs imported from production. Copy the producer helpers too: sharing the new compact builder with the oracle would make the test circular. Split by the exact function boundaries listed by `rg -n '^    (pub )?fn' src/data_flow.rs`; for the long `build_from_refs`, extract its per-file closure and its definition/use phases identically in production and oracle **before** changing either. Save the unmodified source and SHA256 in `.review/dfg-compaction/oracle-source.rs` as evidence; compare moved method bodies after stripping indentation only. No manual simplification of the old algorithm.

```bash
git show 8b2744b3154b3664ffacb23c42f042a67981967a:src/data_flow.rs > .review/dfg-compaction/oracle-source.rs
shasum -a 256 .review/dfg-compaction/oracle-source.rs
```
Expected: 1,519-line baseline DFG source and its recorded checksum. This is an evidence copy, not the executable oracle; source extraction must still compile.

- [ ] **RED — Add exact observations, never `assert_eq!(VarLocation, ...)` alone.** Use bincode over ordered vectors; bincode includes byte spans that Eq ignores. Core assertion code in `src/data_flow/tests/observe.rs`:

```rust
use serde::Serialize;
fn bytes<T: Serialize>(value: &T) -> Vec<u8> {
    bincode::serialize(value).expect("observation serializes")
}
fn assert_bytes<T: Serialize>(name: &str, old: &T, new: &T) {
    assert_eq!(bytes(old), bytes(new), "{name}");
}
fn path_bytes(paths: &[crate::data_flow::FlowPath]) -> Vec<u8> {
    bytes(&paths.iter().map(|p| (&p.edges, &p.cleansed_for)).collect::<Vec<_>>())
}
```

The full storage observation is an ordered tuple: edges; defs `(binding_key, locations)`; uses; forward `(representative, neighbors)`; backward; labels `((representative_from,representative_to),confidence)`; exact labels; RD stats. Convert each map to a `Vec` in native iterator order and encode it; JSON objects cannot encode tuple keys. For the baseline oracle, this is exact code:

```rust
fn old_storage(g: &LegacyDataFlowGraph) -> Vec<u8> {
    bytes(&(
        &g.edges,
        g.defs.iter().collect::<Vec<_>>(),
        g.uses.iter().collect::<Vec<_>>(),
        g.forward.iter().collect::<Vec<_>>(),
        g.backward.iter().collect::<Vec<_>>(),
        g.labels.iter().collect::<Vec<_>>(),
        g.exact_labels.iter().collect::<Vec<_>>(),
        &g.rd_function_stats,
    ))
}
```

For production in PR A the identical function body takes `&DataFlowGraph`. Task 2 changes only its view adapters, and this old function stays frozen. Public-query comparison code is generated by a macro so neither backend delegates a query to the other:

```rust
macro_rules! compare_queries {
    ($old:expr, $new:expr, $locations:expr, $lines:expr) => {{
        let (old, new) = ($old, $new);
        for loc in $locations {
            assert_bytes("forward", &old.forward_reachable(loc), &new.forward_reachable(loc));
            assert_bytes("backward", &old.backward_reachable(loc), &new.backward_reachable(loc));
            assert_bytes("backward labels", &old.backward_reachable_labeled(loc),
                         &new.backward_reachable_labeled(loc));
            assert_bytes("defs", &old.all_defs_of(&loc.file, loc.var_name()),
                         &new.all_defs_of(&loc.file, loc.var_name()));
        }
        for (file, line) in $lines {
            let sources = vec![(file.clone(), *line), (file.clone(), *line)];
            assert_eq!(path_bytes(&old.taint_forward(&sources)), path_bytes(&new.taint_forward(&sources)));
            for (sink_file, sink_line) in $lines {
                assert_bytes("chop", &old.chop(file, *line, sink_file, *sink_line),
                             &new.chop(file, *line, sink_file, *sink_line));
            }
        }
        assert_eq!(path_bytes(&old.taint_forward($lines)), path_bytes(&new.taint_forward($lines)));
        assert_eq!(path_bytes(&old.taint_forward(&[])), path_bytes(&new.taint_forward(&[])));
    }};
}
```

`locations` is the exact-deduplicated **union of every field's key/value locations**, including defs/uses-only and label-only rows; retain a sorted Vec by `ExactVarLocation::from_legacy`, not a legacy BTreeSet. Add copies with shifted byte spans (legacy hit, exact miss), missing path, missing file, `line=0`, and different `function_start_line`. `lines` is the ordered distinct `(file,line)` union plus `("absent.py",0)` and each file's max line + 1. Run each fixture/generator separately to bound all-pairs chop cost. Do not combine the entire fixture tree into one all-pairs graph. There is no claim to exhaust all arbitrary external inputs; the finite fixture-derived domain is exhaustive for these probes.

- [ ] **RED — Add sequential corpus enumeration.** Use this walker; it records every supported-language file and rejects unreadable/invalid UTF-8 source instead of silently skipping it:

```rust
fn fixture_paths(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir).unwrap()
        .map(|e| e.unwrap().path()).collect();
    entries.sort();
    for path in entries {
        let meta = std::fs::symlink_metadata(&path).unwrap();
        assert!(!meta.file_type().is_symlink(), "fixture symlink: {}", path.display());
        if meta.is_dir() { fixture_paths(&path, out); }
        else if prism::languages::Language::from_path(path.to_str().unwrap()).is_some() {
            out.push(path);
        }
    }
}
```

`tests/dfg_compaction.rs` declares `mod common;` and a `#[cfg(feature="dfg-oracle")] mod dfg_compaction;`. `tests/dfg_compaction/mod.rs` declares `mod corpus; mod outputs;`. For each path use `ParsedFile::parse(path, &read_to_string(path).unwrap(), Language::from_path(path).unwrap()).unwrap()`, a one-file BTreeMap, and build **both** `LegacyDataFlowGraph::build` and production `DataFlowGraph::build`. Also run the whole fixture tree as a multi-file build for storage/assembly parity only (no quadratic all-pairs queries on that combined graph).

Invoke every common generator explicitly, retaining its returned DiffInput for output tests:

```rust
let normal = [
    common::make_python_test(), common::make_javascript_test(),
    common::make_go_test(), common::make_java_test(),
    common::make_typescript_test(), common::make_c_test(),
    common::make_c_multifile_test(), common::make_cpp_test(),
    common::make_tsx_test(), common::make_terraform_test(), common::make_bash_test(),
];
let security = [
    common::make_snmp_overflow_test(), common::make_double_free_test(),
    common::make_ring_overflow_test(), common::make_timer_uaf_test(),
    common::make_large_function_test(), common::make_deep_switch_test(),
];
```

Add the explicit Lua case `local function f(x)\n local y = x\n return y\nend\n` at `compact.lua` and a Rust case `fn f(x: i32) -> i32 { let y = x; y }` at `compact.rs`. The helper generators `build_cpg`, `build_python_cpg`, `build_rust_cpg` are wrappers, not additional source corpora; exercise them with these/simple Python sources and compare their CPG dumps too. Assert all 12 `Language` variants are in the observed set. Planning's tracked fixture extension count was 109 files across 9 languages: Bash 1, C 27, Go 20, JS 21, Python 33, Rust 2, Terraform 1, TSX 2, TS 2. Report the execution count; a source-count change needs explanation, not hardcoded exclusion. Java/Cpp come from generators, Lua from the explicit case.

- [ ] **Run RED.**

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features dfg-oracle --test dfg_compaction > .review/dfg-compaction/logs/t1-red.log 2>&1
cat .review/dfg-compaction/logs/t1-red.log
```
Expected: the initial missing oracle module/API compile error. After the extraction, deliberately alter the observed `start_byte` of an equal-legacy location in the candidate only and rerun: the exact-byte oracle must fail. Restore that single test mutant. The compile error alone does not prove byte coverage.

- [ ] **GREEN.** Build both graphs independently, run full ordered storage/query observations, clone, empty graph and `build_subset` for each fixture's file subsets (empty, all, each singleton, plus absent file). Capture legacy serialized bytes before later storage changes. For CPG output parity, add a test-only `assemble_with_dfg` entry that invokes existing `CodePropertyGraph::assemble_graph` with the selected graph; do not change assembly behavior. Initially both sources are legacy. At activation the legacy source is converted through test-only parts, and a separate frozen baseline executable transcript control (Task 6) prevents shared-assembly bugs from passing the dual test.

- [ ] **Gate and commit.** Run GATE; expected all queries/storage bytes equal, all 12 languages exercised, full suites/Tier-A green, byte-span mutant killed. Controller commit title: **`test: freeze DFG behavior and fixture oracle`**, with both required trailers. Refresh handoff. Production memory effect: **derived 0 MiB**.

### Task 2 (PR B): Ordered views and mechanical consumer migration

**Files:** `src/data_flow/{types,views,queries,legacy_storage}.rs`, producer child modules, named consumer paths in the inventory; extract CPG assembly into `src/cpg/dfg_assembly.rs`; test child modules. No storage/serde/layout change.

**Interfaces:** `BindingKey = (String, String, usize, AccessPath)`; borrowed edge/location views; existing public graph queries retain their exact signatures. `DataFlowGraph` initially keeps the old eight serialized fields. New methods are not serialized. Oracle retains direct legacy fields and its independent query bodies.

- [ ] **RED — Test the accessor order against the old fields.** Create `src/data_flow/tests/queries.rs`:

```rust
#[test]
fn views_include_spans_and_preserve_native_order() {
    let files = fixture_python("def f(x):\n y = x + x\n return y\n");
    let g = DataFlowGraph::build(&files);
    let old = LegacyDataFlowGraph::build(&files);
    let actual: Vec<_> = g.edges().iter().map(|e| e.to_owned()).collect();
    assert_eq!(bincode::serialize(&old.edges).unwrap(), bincode::serialize(&actual).unwrap());
    let actual: Vec<_> = g.defs().map(|(k, v)| (k, v.to_vec())).collect();
    let expected: Vec<_> = old.defs.iter().collect();
    assert_eq!(bincode::serialize(&expected).unwrap(), bincode::serialize(&actual).unwrap());
    assert_eq!(g.edges().len(), old.edges.len());
    assert_eq!(g.edges().is_empty(), old.edges.is_empty());
}
fn fixture_python(source: &str) -> BTreeMap<String, ParsedFile> {
    BTreeMap::from([("fixture.py".into(), ParsedFile::parse(
        "fixture.py", source, crate::languages::Language::Python).unwrap())])
}
```

Run the focused test with the standard wrapped cargo command, logging to `t2-red.log`; Expected: missing view API before the implementation, then behavioral parity after it. Run command:

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --lib views_include_spans_and_preserve_native_order > .review/dfg-compaction/logs/t2-red.log 2>&1
cat .review/dfg-compaction/logs/t2-red.log
```
Expected: the named test is selected (or its named API compile RED); zero selected is inadmissible.

- [ ] **GREEN — Add these exact boundary types and legacy implementations.** In `views.rs` import `super::{DataFlowGraph, FlowEdge, VarLocation, AccessPath}`. The boxed iterators own only iterator state, never expanded graph records. `Locations` is deliberately not Clone-as-Vec; call `to_vec()` where the old query cloned a neighbor vector.

```rust
pub type BindingKey = (String, String, usize, AccessPath);
#[derive(Clone, Copy)]
pub struct FlowEdgeRef<'a> {
    pub from: &'a VarLocation,
    pub to: &'a VarLocation,
}
impl FlowEdgeRef<'_> {
    pub fn to_owned(self) -> FlowEdge {
        FlowEdge { from: self.from.clone(), to: self.to.clone() }
    }
}
pub struct Edges<'a> {
    pub(crate) len: usize,
    pub(crate) iter: Box<dyn Iterator<Item = FlowEdgeRef<'a>> + 'a>,
}
impl<'a> Edges<'a> {
    pub fn len(&self) -> usize { self.len }
    pub fn is_empty(&self) -> bool { self.len == 0 }
    pub fn iter(self) -> Box<dyn Iterator<Item = FlowEdgeRef<'a>> + 'a> { self.iter }
}
pub struct Locations<'a> {
    pub(crate) iter: Box<dyn Iterator<Item = &'a VarLocation> + 'a>,
}
impl<'a> Locations<'a> {
    pub fn iter(self) -> Box<dyn Iterator<Item = &'a VarLocation> + 'a> { self.iter }
    pub fn to_vec(self) -> Vec<VarLocation> { self.iter.cloned().collect() }
}
impl<'a> IntoIterator for Locations<'a> {
    type Item = &'a VarLocation;
    type IntoIter = Box<dyn Iterator<Item = Self::Item> + 'a>;
    fn into_iter(self) -> Self::IntoIter { self.iter }
}
impl DataFlowGraph {
    pub fn edges(&self) -> Edges<'_> {
        Edges { len: self.edges.len(), iter: Box::new(self.edges.iter()
            .map(|e| FlowEdgeRef { from: &e.from, to: &e.to })) }
    }
    pub fn defs(&self) -> impl Iterator<Item = (&BindingKey, Locations<'_>)> {
        self.defs.iter().map(|(k, v)| (k, Locations { iter: Box::new(v.iter()) }))
    }
    pub fn uses(&self) -> impl Iterator<Item = (&BindingKey, Locations<'_>)> {
        self.uses.iter().map(|(k, v)| (k, Locations { iter: Box::new(v.iter()) }))
    }
    pub fn forward(&self) -> impl Iterator<Item = (&VarLocation, Locations<'_>)> {
        self.forward.iter().map(|(k, v)| (k, Locations { iter: Box::new(v.iter()) }))
    }
    pub fn backward(&self) -> impl Iterator<Item = (&VarLocation, Locations<'_>)> {
        self.backward.iter().map(|(k, v)| (k, Locations { iter: Box::new(v.iter()) }))
    }
    pub fn successors(&self, at: &VarLocation) -> Option<Locations<'_>> {
        self.forward.get(at).map(|v| Locations { iter: Box::new(v.iter()) })
    }
    pub fn predecessors(&self, at: &VarLocation) -> Option<Locations<'_>> {
        self.backward.get(at).map(|v| Locations { iter: Box::new(v.iter()) })
    }
    pub fn label(&self, from: &VarLocation, to: &VarLocation) -> Option<crate::cpg::FlowConfidence> {
        self.labels.get(&(from.clone(), to.clone())).copied()
    }
    pub fn labels(&self) -> impl Iterator<Item = (FlowEdgeRef<'_>, crate::cpg::FlowConfidence)> {
        self.labels.iter().map(|((from, to), &label)| (FlowEdgeRef { from, to }, label))
    }
    pub(crate) fn exact_labels(&self) -> impl Iterator<Item = (super::ExactFlowEdge, crate::cpg::FlowConfidence)> + '_ {
        self.exact_labels.iter().map(|(edge, &label)| (edge.clone(), label))
    }
    pub fn rd_function_stats(&self) -> &std::collections::BTreeMap<String, crate::cpg::RdFileStats> {
        &self.rd_function_stats
    }
    pub(crate) fn rd_function_stats_mut(&mut self) -> &mut std::collections::BTreeMap<String, crate::cpg::RdFileStats> {
        &mut self.rd_function_stats
    }
}
```

Export `BindingKey`, `FlowEdgeRef`, `Edges`, `Locations` from `data_flow.rs`. `exact_labels` stays crate-internal. Integration oracle entry points expose observation bytes, not the internal exact DTO.

- [ ] **GREEN — Move, then adapt the seven query bodies exactly.** Move baseline `forward_reachable`, `backward_reachable`, `backward_reachable_labeled`, `chop`, `all_locations_at`, `taint_forward`, `all_defs_of` into `queries.rs`. The only edits inside them are this closed substitution set:

| Old expression | New expression |
|---|---|
| `self.forward.get(&loc)` | `self.successors(&loc)` |
| `self.backward.get(&loc)` | `self.predecessors(&loc)` |
| `let mut prevs = prevs.clone();` | `let mut prevs = prevs.to_vec();` |
| `for ((f, func, fsl, _path), def_locs) in &self.defs` | `for ((f, func, fsl, _path), def_locs) in self.defs()` |
| `self.labels.get(&(prev.clone(), loc.clone()))` and pattern `Some(&confidence)` | `self.label(prev, &loc)` and pattern `Some(confidence)` |
| `for locs in self.defs.values()` / uses | `for (_, locs) in self.defs()` / `self.uses()` |
| `for ((f, _func, _fsl, path), locs) in &self.defs` | `for ((f, _func, _fsl, path), locs) in self.defs()` |
| `result.extend(locs.clone());` | `result.extend(locs.iter().cloned());` |

Keep `VecDeque`, `visited: BTreeSet<VarLocation>`, `prevs.sort(); prevs.dedup();`, the initial caller-supplied location, missing-label behavior, and source/sink inclusion exactly as written. Do not optimize traversal to IDs in this lane: it would change which equal-legacy occurrence wins. `FlowPath` continues to contain owned `FlowEdge`s.

- [ ] **GREEN — Migrate every consumer in the inventory.** Exact transformations: `.edges.iter()` → `.edges().iter()`; `&dfg.edges` → `dfg.edges().iter()`; `.edges.is_empty()` → `.edges().is_empty()`; `.defs.values()` → `.defs().map(|(_, v)| v)`; corresponding uses; label lookup → `label(&from,&to)` without `.copied()`; `ExactFlowEdge::from_legacy(edge)` → `ExactFlowEdge::from_legacy(&edge.to_owned())` at the assembly conflict-index construction only; exact iterator patterns change from borrowed `&confidence` to value `confidence`. Preserve ordered key iteration and each closure's body. Synthetic field mutation remains in unit-test-only `edit_legacy_parts`: convert a **small test graph**, apply the supplied closure on old Parts, freeze it again; never enable that hook in release binaries. Existing integration tests that need this hook must run under `dfg-oracle`; normal tests should use a crate-internal unit test or a constructor from source, not silently disappear under default/mcp suites.

```bash
rg -n 'DataFlowGraph|\b(cached_dfg|fresh_dfg|dfg|file_refs)\.(edges|defs|uses|forward|backward|labels|exact_labels|rd_function_stats)' src tests > .review/dfg-compaction/logs/t2-consumers.txt
cat .review/dfg-compaction/logs/t2-consumers.txt
```
Expected: all non-builder production storage uses go through the named views; remaining raw fields are legacy storage, frozen oracle and explicitly temporary test fixtures. Repeat a broader multiline `rg -U '\.dfg\s*\.\s*(edges|defs|uses|labels)' src tests` and review aliases manually. Expected: no unaccounted raw consumer. No `src/reasoning` or taint algorithm rewrite.

- [ ] **Gate and commit.** Run GATE, including oracle output snapshots before/after method moves. Mutant: reverse `edges()` or choose the last equal-legacy representative; ordered observations must fail. Controller commit title **`refactor: isolate ordered DFG accessors`**, both trailers. Split extraction-only B1/B2 if the predeclared budget requires it. Memory effect **derived 0 MiB**, no cache bump.

### Task 3 (PR C): Immutable exact arena and compact read storage

**Files:** `src/data_flow/compact/{mod,arena,freeze,views}.rs`, category tests. Initially compiled only for tests/`dfg-oracle`; the production facade remains legacy until Task 6.

**Interfaces:** `OccId(u32)` addresses one exact arena row; `LegacyId(u32)` addresses a rank in the legacy index. IDs never cross a graph lifetime, output boundary or unvalidated deserializer. `CompactDfg::from_parts(Parts) -> Result<Self, &'static str>` consumes owned primary state. `Parts` is the old field layout from Task 1 with optional preserved adjacency (below). Create `compact/queries.rs` by copying Task 2's migrated `queries.rs` and changing only `impl DataFlowGraph` to `impl CompactDfg` plus its import; this supplies all seven query methods to the test-only backend. Keep it test-only after activation to avoid duplicate production algorithms. Oracle bodies remain frozen.

- [ ] **RED — Add the identity/representative tests first.** The compiled arena/insertion kernel and test bodies follow; static self-review strengthened the neighbor-order case to an asymmetric four-element sequence. That final strengthened test awaits its linked Task 3 run. The plan's standalone validation used the real source DTOs and confidence lattice (serde derives removed), not a hand-invented Eq. Move these tests into `identity.rs` and `representatives.rs`; imports are `super::*` plus existing DTO/lattice types. The helper `arena(rows)` sorts/deduplicates exact rows **only in tests**; production `from_sorted` rejects noncanonical decoded arenas.

**Arena implementation in `compact/arena.rs`:**

```rust
use super::{VarLocation, VarAccessKind, AccessPath};
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct OccId(pub u32);
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct LegacyId(pub u32);

pub fn checked_id(n: usize) -> Result<u32, &'static str> {
    u32::try_from(n).map_err(|_| "DFG arena exceeds u32 ID space")
}
pub fn exact_key(v: &VarLocation) -> (&str, &str, usize, usize, &AccessPath, usize, usize, VarAccessKind) {
    (&v.file, &v.function, v.function_start_line, v.line, &v.path,
     v.start_byte, v.end_byte, v.kind)
}
pub fn exact_cmp(a: &VarLocation, b: &VarLocation) -> Ordering {
    exact_key(a).cmp(&exact_key(b))
}
#[derive(Debug, Clone)]
pub struct Arena {
    pub rows: Vec<VarLocation>,
    pub legacy_reps: Vec<OccId>,
    pub legacy_by_occ: Vec<LegacyId>,
}
impl Arena {
    pub fn from_sorted(rows: Vec<VarLocation>) -> Result<Self, &'static str> {
        checked_id(rows.len())?;
        if rows.windows(2).any(|w| exact_cmp(&w[0], &w[1]) != Ordering::Less) {
            return Err("DFG arena is not strictly exact-sorted");
        }
        let mut legacy_reps: Vec<_> = (0..rows.len()).map(|i| OccId(i as u32)).collect();
        legacy_reps.sort_by(|a, b| rows[a.0 as usize].cmp(&rows[b.0 as usize]));
        legacy_reps.dedup_by(|b, a| rows[a.0 as usize] == rows[b.0 as usize]);
        let legacy_by_occ = rows.iter().map(|v| {
            let i = legacy_reps.binary_search_by(|r| rows[r.0 as usize].cmp(v)).unwrap();
            LegacyId(i as u32)
        }).collect();
        Ok(Self { rows, legacy_reps, legacy_by_occ })
    }
    pub fn get(&self, id: OccId) -> &VarLocation { &self.rows[id.0 as usize] }
    pub fn exact(&self, v: &VarLocation) -> Option<OccId> {
        self.rows.binary_search_by(|row| exact_cmp(row, v)).ok().map(|i| OccId(i as u32))
    }
    pub fn legacy(&self, v: &VarLocation) -> Option<LegacyId> {
        self.legacy_reps.binary_search_by(|r| self.get(*r).cmp(v)).ok().map(|i| LegacyId(i as u32))
    }
    pub fn group(&self, id: OccId) -> LegacyId { self.legacy_by_occ[id.0 as usize] }
}
```

**Row insertion kernel and tests (the asymmetric neighbor test is strengthened after the standalone run; row helpers belong in `compact/mod.rs`, tests in the category modules):**

```rust
#[derive(Debug, Clone)]
struct Adj { representative: OccId, neighbors: Vec<OccId> }
#[derive(Debug, Clone)]
struct Label { from: OccId, to: OccId, confidence: FlowConfidence }
fn add_adj(map: &mut BTreeMap<LegacyId, Adj>, a: &Arena, key: OccId, val: OccId) {
    map.entry(a.group(key)).or_insert_with(|| Adj { representative: key, neighbors: Vec::new() }).neighbors.push(val);
}
fn add_label(map: &mut BTreeMap<(LegacyId, LegacyId), Label>, a: &Arena,
    from: OccId, to: OccId, confidence: FlowConfidence) {
    map.entry((a.group(from), a.group(to)))
        .and_modify(|old| old.confidence = old.confidence.worst(confidence))
        .or_insert(Label { from, to, confidence });
}
fn loc(name: &str, kind: VarAccessKind, start: usize) -> VarLocation {
    VarLocation { file: "u.py".into(), function: "f".into(), function_start_line: 1,
        line: 2, path: AccessPath { base: name.into(), fields: vec![] },
        start_byte: start, end_byte: start + 1, kind }
}
fn arena(mut rows: Vec<VarLocation>) -> Arena {
    rows.sort_by(exact_cmp);
    rows.dedup_by(|b, a| exact_cmp(a, b).is_eq());
    Arena::from_sorted(rows).unwrap()
}
#[test] fn legacy_identity_excludes_bytes_exact_identity_does_not() {
    let a=loc("x",VarAccessKind::Use,10); let b=loc("x",VarAccessKind::Use,30);
    let g=arena(vec![a.clone(),b.clone()]);
    assert_eq!(a,b); assert_eq!(g.legacy(&a),g.legacy(&b)); assert_ne!(g.exact(&a),g.exact(&b));
}
#[test] fn kind_precedes_bytes_only_in_legacy_order() {
    let d=loc("x",VarAccessKind::Def,30); let u=loc("x",VarAccessKind::Use,10);
    let g=arena(vec![d.clone(),u.clone()]);
    assert!(g.exact(&u)<g.exact(&d)); assert!(g.legacy(&d)<g.legacy(&u));
}
#[test] fn map_representatives_are_independent() {
    let d0=loc("x",VarAccessKind::Def,10); let d1=loc("x",VarAccessKind::Def,20);
    let u=loc("x",VarAccessKind::Use,30); let g=arena(vec![d0.clone(),d1.clone(),u.clone()]);
    let (d0,d1,u)=(g.exact(&d0).unwrap(),g.exact(&d1).unwrap(),g.exact(&u).unwrap());
    let mut adj=BTreeMap::new(); let mut labels=BTreeMap::new();
    add_adj(&mut adj,&g,d1,u); add_label(&mut labels,&g,d0,u,FlowConfidence::Exact);
    assert_eq!(g.get(adj.values().next().unwrap().representative).start_byte,20);
    assert_eq!(g.get(labels.values().next().unwrap().from).start_byte,10);
}
#[test] fn adjacency_retains_duplicates_and_input_order() {
    let d=loc("x",VarAccessKind::Def,1); let u0=loc("x",VarAccessKind::Use,10); let u1=loc("x",VarAccessKind::Use,20);
    let g=arena(vec![d.clone(),u0.clone(),u1.clone()]);
    let (d,u0,u1)=(g.exact(&d).unwrap(),g.exact(&u0).unwrap(),g.exact(&u1).unwrap());
    let mut adj=BTreeMap::new(); for u in [u1,u0,u1,u0] { add_adj(&mut adj,&g,d,u); }
    assert_eq!(adj.values().next().unwrap().neighbors,vec![u1,u0,u1,u0]);
}
#[test] fn labels_keep_first_pair_and_merge_worst() {
    let d=loc("x",VarAccessKind::Def,1); let u0=loc("x",VarAccessKind::Use,10); let u1=loc("x",VarAccessKind::Use,20);
    let g=arena(vec![d.clone(),u0.clone(),u1.clone()]);
    let (d,u0,u1)=(g.exact(&d).unwrap(),g.exact(&u0).unwrap(),g.exact(&u1).unwrap());
    let mut labels=BTreeMap::new(); add_label(&mut labels,&g,d,u1,FlowConfidence::Exact);
    add_label(&mut labels,&g,d,u0,FlowConfidence::NameOnly(FlowDoubt::Killed{kill_line:7}));
    add_label(&mut labels,&g,d,u0,FlowConfidence::NameOnly(FlowDoubt::Killed{kill_line:3}));
    let v=labels.values().next().unwrap(); assert_eq!(v.to,u1);
    assert_eq!(v.confidence,FlowConfidence::NameOnly(FlowDoubt::Killed{kill_line:3}));
}
#[test] fn unknown_exact_query_can_still_find_legacy_key() {
    let a=loc("x",VarAccessKind::Use,10); let b=loc("x",VarAccessKind::Use,900);
    let g=arena(vec![a]); assert!(g.exact(&b).is_none()); assert!(g.legacy(&b).is_some());
}
#[test] fn duplicate_or_unsorted_cache_arena_is_rejected() {
    let a=loc("x",VarAccessKind::Use,10); let b=loc("x",VarAccessKind::Use,20);
    assert!(Arena::from_sorted(vec![a.clone(),a.clone()]).is_err());
    assert!(Arena::from_sorted(vec![b,a]).is_err());
}
#[test] fn empty_arena_has_no_matches() {
    let g=Arena::from_sorted(vec![]).unwrap(); assert!(g.legacy(&loc("x",VarAccessKind::Def,1)).is_none());
}
#[test] fn id_boundary_is_checked_without_allocating_billions() {
    assert_eq!(checked_id(u32::MAX as usize),Ok(u32::MAX));
    assert!(checked_id(u32::MAX as usize + 1).is_err());
}
#[test] fn equal_legacy_neighbors_stable_sort_keeps_first_span() {
    let mut vs=vec![loc("x",VarAccessKind::Use,20),loc("x",VarAccessKind::Use,10)];
    vs.sort(); vs.dedup(); assert_eq!(vs.len(),1); assert_eq!(vs[0].start_byte,20);
}
```

`compact/mod.rs` imports `VarLocation`, `VarAccessKind`, `AccessPath`, `FlowConfidence`, `FlowDoubt` from the existing modules, declares `mod arena;`, and imports `arena::*`. Production visibility of IDs/fields is `pub(super)` or private; the displayed `pub` in the standalone kernel is not an external ID API. `checked_id(rows.len())` intentionally caps the arena at `u32::MAX` rows rather than using the last representable ID; this reserves no sentinel in the wire and prevents `len` overflow assumptions.

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features dfg-oracle --lib data_flow::tests > .review/dfg-compaction/logs/t3-red.log 2>&1
cat .review/dfg-compaction/logs/t3-red.log
```
Expected: before kernel implementation, missing arena API; with an intentionally legacy-Eq interner, the distinct-span test fails; with occurrence ordinal used as legacy order, the Def/Use ordering test fails. Restore mutants; all ten named kernel tests pass. The standalone receipt does not replace this linked run.

- [ ] **GREEN — Add compact field types.** `Adj` and `Label` are the exact types above; make them module-private. `IdEdge` is `(OccId,OccId)`, not a struct with an unchecked cast.

```rust
type IdEdge = (OccId, OccId);
#[derive(Debug, Clone)]
pub(super) struct CompactDfg {
    arena: Arena,
    edges: Vec<IdEdge>,
    defs: BTreeMap<BindingKey, Vec<OccId>>,
    uses: BTreeMap<BindingKey, Vec<OccId>>,
    forward: BTreeMap<LegacyId, Adj>,
    backward: BTreeMap<LegacyId, Adj>,
    labels: BTreeMap<(LegacyId, LegacyId), Label>,
    exact_labels: BTreeMap<IdEdge, FlowConfidence>,
    rd_function_stats: BTreeMap<String, crate::cpg::RdFileStats>,
}
#[derive(Clone)]
pub(super) struct Parts {
    pub edges: Vec<FlowEdge>,
    pub defs: BTreeMap<BindingKey, Vec<VarLocation>>,
    pub uses: BTreeMap<BindingKey, Vec<VarLocation>>,
    pub labels: BTreeMap<(VarLocation, VarLocation), FlowConfidence>,
    pub exact_labels: BTreeMap<ExactFlowEdge, FlowConfidence>,
    pub rd_function_stats: BTreeMap<String, crate::cpg::RdFileStats>,
    pub adjacency: Option<(
        BTreeMap<VarLocation, Vec<VarLocation>>,
        BTreeMap<VarLocation, Vec<VarLocation>>,
    )>,
}
```

`adjacency=Some` is for faithfully importing a legacy graph in tests, including map-specific representatives and non-edge rows. `None` means the normal producer never built expanded adjacency; rebuild over compact edge IDs. Never silently rebuild when `Some` was supplied: that masks an ordering/representative discrepancy.

- [ ] **GREEN — Implement consuming interning and rank remap with this code.** In `freeze.rs`, `TempId` is `OccId` used only before freeze. Exact keys own location payload once; the map is consumed and dropped before publication. No retained lookup dictionary beside the frozen arena.

```rust
#[derive(Default)]
struct Intern { rows: BTreeMap<ExactVarLocation, OccId> }
fn own_exact(v: VarLocation) -> ExactVarLocation {
    ExactVarLocation { file: v.file, function: v.function,
        function_start_line: v.function_start_line, line: v.line, path: v.path,
        start_byte: v.start_byte, end_byte: v.end_byte, kind: v.kind }
}
fn own_legacy(v: ExactVarLocation) -> VarLocation {
    VarLocation { file: v.file, function: v.function,
        function_start_line: v.function_start_line, line: v.line, path: v.path,
        start_byte: v.start_byte, end_byte: v.end_byte, kind: v.kind }
}
impl Intern {
    fn put(&mut self, v: VarLocation) -> Result<OccId, &'static str> {
        let key = own_exact(v);
        if let Some(&id) = self.rows.get(&key) { return Ok(id); }
        let next = self.rows.len().checked_add(1).ok_or("DFG arena capacity overflow")?;
        checked_id(next)?;
        let id = OccId(checked_id(self.rows.len())?);
        self.rows.insert(key,id);
        Ok(id)
    }
    fn finish(self) -> Result<(Arena, Vec<OccId>), &'static str> {
        let mut remap = vec![OccId(0); self.rows.len()];
        let mut rows = Vec::with_capacity(self.rows.len());
        for (key, old) in self.rows {
            remap[old.0 as usize] = OccId(checked_id(rows.len())?);
            rows.push(own_legacy(key));
        }
        Ok((Arena::from_sorted(rows)?, remap))
    }
}
fn intern_bindings(m: BTreeMap<BindingKey, Vec<VarLocation>>, i: &mut Intern)
    -> Result<BTreeMap<BindingKey, Vec<OccId>>, &'static str> {
    m.into_iter().map(|(k, vs)| {
        Ok((k, vs.into_iter().map(|v| i.put(v)).collect::<Result<_,_>>()?))
    }).collect()
}
fn intern_adj(m: BTreeMap<VarLocation, Vec<VarLocation>>, i: &mut Intern)
    -> Result<Vec<Adj>, &'static str> {
    m.into_iter().map(|(k, vs)| Ok(Adj { representative: i.put(k)?,
        neighbors: vs.into_iter().map(|v| i.put(v)).collect::<Result<_,_>>()? })).collect()
}
fn remap_bindings(m: &mut BTreeMap<BindingKey, Vec<OccId>>, r: &[OccId]) {
    for vs in m.values_mut() { for id in vs { *id = r[id.0 as usize]; } }
}
fn remap_adj(rows: Vec<Adj>, a: &Arena, r: &[OccId]) -> BTreeMap<LegacyId, Adj> {
    rows.into_iter().map(|mut row| {
        row.representative = r[row.representative.0 as usize];
        for id in &mut row.neighbors { *id = r[id.0 as usize]; }
        (a.group(row.representative), row)
    }).collect()
}
impl CompactDfg {
    pub(super) fn from_parts(p: Parts) -> Result<Self, &'static str> {
        let mut i = Intern::default();
        let mut edges: Vec<IdEdge> = p.edges.into_iter()
            .map(|e| Ok((i.put(e.from)?, i.put(e.to)?)))
            .collect::<Result<_, &'static str>>()?;
        let mut defs = intern_bindings(p.defs, &mut i)?;
        let mut uses = intern_bindings(p.uses, &mut i)?;
        let labels: Vec<Label> = p.labels.into_iter().map(|((from,to), confidence)|
            Ok(Label { from: i.put(from)?, to: i.put(to)?, confidence }))
            .collect::<Result<_, &'static str>>()?;
        let exact: Vec<Label> = p.exact_labels.into_iter().map(|(e, confidence)|
            Ok(Label { from: i.put(own_legacy(e.from))?,
                to: i.put(own_legacy(e.to))?, confidence }))
            .collect::<Result<_, &'static str>>()?;
        let adjacency = match p.adjacency {
            Some((f,b)) => Some((intern_adj(f, &mut i)?, intern_adj(b, &mut i)?)),
            None => None,
        };
        let (arena, r) = i.finish()?;
        for (from,to) in &mut edges { *from = r[from.0 as usize]; *to = r[to.0 as usize]; }
        remap_bindings(&mut defs, &r); remap_bindings(&mut uses, &r);
        let (forward, backward) = match adjacency {
            Some((f,b)) => (remap_adj(f, &arena, &r), remap_adj(b, &arena, &r)),
            None => {
                let (mut f, mut b) = (BTreeMap::new(), BTreeMap::new());
                for &(from,to) in &edges {
                    add_adj(&mut f, &arena, from, to); add_adj(&mut b, &arena, to, from);
                }
                (f,b)
            }
        };
        let mut reduced = BTreeMap::new();
        for l in labels { add_label(&mut reduced, &arena,
            r[l.from.0 as usize], r[l.to.0 as usize], l.confidence); }
        let mut exact_labels = BTreeMap::new();
        for l in exact {
            exact_labels.entry((r[l.from.0 as usize],r[l.to.0 as usize]))
                .and_modify(|v: &mut FlowConfidence| *v = v.worst(l.confidence))
                .or_insert(l.confidence);
        }
        Ok(Self { arena, edges, defs, uses, forward, backward, labels: reduced,
            exact_labels, rd_function_stats: p.rd_function_stats })
    }
}
```

The arena contains locations from **all** these fields, including independent adjacency keys and labels absent from edges. Binding keys with empty value vectors survive. `from_parts` is used for per-file production staging and test conversion, **never a decoded full legacy cache**.

- [ ] **GREEN — Implement the compact views.** Use Task 2 view types; replace backing collections, not query logic. The following is the complete edge/lookup boundary; defs/uses/map iterators use the explicit mapping functions immediately below it.

```rust
impl CompactDfg {
    fn locations<'a>(&'a self, ids: &'a [OccId]) -> Locations<'a> {
        Locations { iter: Box::new(ids.iter().map(|id| self.arena.get(*id))) }
    }
    fn edges(&self) -> Edges<'_> {
        Edges { len: self.edges.len(), iter: Box::new(self.edges.iter().map(|&(f,t)|
            FlowEdgeRef { from: self.arena.get(f), to: self.arena.get(t) })) }
    }
    fn defs(&self) -> impl Iterator<Item=(&BindingKey, Locations<'_>)> {
        self.defs.iter().map(|(k,v)| (k,self.locations(v)))
    }
    fn uses(&self) -> impl Iterator<Item=(&BindingKey, Locations<'_>)> {
        self.uses.iter().map(|(k,v)| (k,self.locations(v)))
    }
    fn forward(&self) -> impl Iterator<Item=(&VarLocation, Locations<'_>)> {
        self.forward.values().map(|v| (self.arena.get(v.representative),self.locations(&v.neighbors)))
    }
    fn backward(&self) -> impl Iterator<Item=(&VarLocation, Locations<'_>)> {
        self.backward.values().map(|v| (self.arena.get(v.representative),self.locations(&v.neighbors)))
    }
    fn successors(&self, at: &VarLocation) -> Option<Locations<'_>> {
        self.forward.get(&self.arena.legacy(at)?).map(|v| self.locations(&v.neighbors))
    }
    fn predecessors(&self, at: &VarLocation) -> Option<Locations<'_>> {
        self.backward.get(&self.arena.legacy(at)?).map(|v| self.locations(&v.neighbors))
    }
    fn label(&self, from: &VarLocation, to: &VarLocation) -> Option<FlowConfidence> {
        self.labels.get(&(self.arena.legacy(from)?,self.arena.legacy(to)?)).map(|v| v.confidence)
    }
    fn labels(&self) -> impl Iterator<Item=(FlowEdgeRef<'_>,FlowConfidence)> {
        self.labels.values().map(|v| (FlowEdgeRef { from: self.arena.get(v.from),
            to: self.arena.get(v.to) },v.confidence))
    }
    fn exact_labels(&self) -> impl Iterator<Item=(ExactFlowEdge,FlowConfidence)> + '_ {
        self.exact_labels.iter().map(|(&(f,t),&c)| (ExactFlowEdge {
            from: ExactVarLocation::from_legacy(self.arena.get(f)),
            to: ExactVarLocation::from_legacy(self.arena.get(t)) },c))
    }
    fn rd_function_stats(&self) -> &BTreeMap<String,crate::cpg::RdFileStats> {
        &self.rd_function_stats
    }
}
```

No persistent `FlowEdge` view cache. Exact-label iteration creates only the yielded owned DTO (rare supplemental facts), not a second stored map. Production façade forwarding methods expose the same visibility as Task 2; compact methods are `pub(super)` where the parent wrapper needs them.

- [ ] **GREEN — Run both representations on every Task 1 corpus case.** Compact source is consumed legacy Parts for conversion parity, and a separately built producer result for construction parity. Run all public queries and ordered storage observations. Compare bincode bytes, not legacy Eq. Add the ten kernel mutants to the gate catalog: collapse bytes; use numeric OccId as LegacyId; replace map representative; sort/dedup adjacency; choose latest label pair; replace `worst` by `max`; require exact lookup for a legacy query; accept unsorted/duplicate arena; wrapping ID cast; unstable-sort equal neighbors.

- [ ] **Gate and commit.** GATE; expected no public difference across corpus, kernel tests plus mutants prove distinctions, default/mcp production still legacy. Controller commit title **`feat: add compact DFG arena behind test oracle`**, both trailers. Runtime effect **derived 0 MiB**. Retain provisional compact accounting for Task 4's measurement checkpoint.

### Task 4 (PR C): Incremental removal, merge and reclamation over IDs

**Files:** `compact/update.rs`, `tests/updates.rs`; no production activation yet.

**Interfaces:** `CompactDfg::{remove_files(&mut self,&BTreeSet<String>), merge(&mut self,Self)}` match the old signatures. `combine(Vec<CompactDfg>) -> Result<CompactDfg,&'static str>` handles ordered parts in one pass; no repeated pairwise merge over every file. `remove_files` rebuilds adjacency even for an empty exclusion set, matching baseline `src/data_flow.rs:235-255`.

- [ ] **RED — Add this exact operation-sequence oracle.** `pair(parts)` constructs a legacy graph and compact graph from the same independently copied Parts; `assert_all(old,new)` is Task 1's storage/public-query oracle. Helpers are test-only and never used by production.

```rust
#[test]
fn overlap_merge_and_remove_match_legacy_bytes() {
    let (mut old, mut new) = pair(adversarial_parts());
    assert_all(&old, &new);
    let (rhs_old, rhs_new) = pair(adversarial_parts());
    old.merge(rhs_old); new.merge(rhs_new);
    assert_all(&old, &new);
    // Empty removal still rebuilds both adjacency maps.
    old.remove_files(&BTreeSet::new()); new.remove_files(&BTreeSet::new());
    assert_all(&old, &new);
    for excluded in [BTreeSet::from(["a.py".into()]),
                     BTreeSet::from(["absent.py".into()]),
                     BTreeSet::from(["b.py".into()])] {
        old.remove_files(&excluded); new.remove_files(&excluded);
        assert_all(&old, &new);
    }
}
#[test]
fn twenty_refreshes_reclaim_dead_locations_and_leave_clone_unchanged() {
    let files = fixture_python("def f(x):\n y=x\n return y\n");
    let (mut old, mut new) = built_pair(&files);
    let held = new.clone(); let held_bytes = compact_observation(&held);
    let initial = new.arena.rows.len();
    for generation in 0..20 {
        let file = format!("changed_{generation}.py");
        let parsed = ParsedFile::parse(&file, "def g(a):\n return a\n",
            crate::languages::Language::Python).unwrap();
        let files = BTreeMap::from([(file.clone(),parsed)]);
        let (part_old, part_new) = built_pair(&files);
        old.merge(part_old); new.merge(part_new);
        assert_all(&old,&new);
        let exclude = BTreeSet::from([file]);
        old.remove_files(&exclude); new.remove_files(&exclude);
        assert_all(&old,&new);
        assert_eq!(new.arena.rows.len(),initial);
        assert_eq!(compact_observation(&held),held_bytes);
        assert!(new.arena.rows.iter().all(|v| !v.file.starts_with("changed_")));
    }
}
```

`adversarial_parts()` is explicit input, not random generation: two files `a.py`,`b.py`; `loc("x",Def,10)`, its byte variant at 20, `loc("x",Use,30)`, its byte variant at 40; set files on copies. Edges in order: `(a:def20,a:use40)`, `(a:def10,a:use30)`, duplicate first edge, `(a:def20,b:use30)`. Defs key `("a.py","f",1,AccessPath::simple("x"))` contains both def occurrences in reverse byte order; uses has the corresponding uses in reverse order. Add an empty defs key for `b.py`; a defs key `b.py` containing an `a.py` location to prove **key-file**, not value-file, removal. Labels insert byte10→byte30 with Exact first, then equal-legacy byte20→byte40 with Killed(7), then Killed(3): assert first pair bytes and Killed(3). Add an edge-absent label at `b.py` and an exact-only byte40 label. Override forward key representative to byte10 while edges first use byte20; preserve this difference before the first update. RD stats: record `("f",1)` in both inputs so merge keeps the distinct identity count, not its arithmetic sum.

Write that fixture with the DTO constructors shown in Task 3, ordinary BTreeMap `entry` insertion, and `RdFileStats::record_without_cfg`; assert its expected storage bytes against hand-selected representatives **before** using it as an oracle input. `removal_uses_map_key_file_and_either_edge_endpoint` removes `b.py` and asserts the cross-file edge, b-key/a-value defs row, b-only label and exact label are gone, while `a.py` rows remain.

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features dfg-oracle --lib data_flow::tests::updates > .review/dfg-compaction/logs/t4-red.log 2>&1
cat .review/dfg-compaction/logs/t4-red.log
```
Expected: missing update methods initially; a tombstone/no-GC implementation fails the 20-refresh arena count; overwriting instead of concatenating overlapping defs/uses fails the operation oracle; excluding only an edge's source fails cross-file removal.

- [ ] **GREEN — Filter the same populations, then combine compact rows.** The core removal is exactly:

```rust
fn filter_primary(g: &mut CompactDfg, excluded: &BTreeSet<String>) {
    let a = &g.arena;
    g.edges.retain(|&(f,t)| !excluded.contains(&a.get(f).file) && !excluded.contains(&a.get(t).file));
    g.defs.retain(|k,_| !excluded.contains(&k.0));
    g.uses.retain(|k,_| !excluded.contains(&k.0));
    g.labels.retain(|_,v| !excluded.contains(&a.get(v.from).file) && !excluded.contains(&a.get(v.to).file));
    g.exact_labels.retain(|&(f,t),_| !excluded.contains(&a.get(f).file) && !excluded.contains(&a.get(t).file));
    g.rd_function_stats.retain(|f,_| !excluded.contains(f));
    g.forward.clear(); g.backward.clear();
}
```

The combine kernel uses temporary IDs only; all inputs are consumed in their incoming order. It interns **live arena rows once each**, not each occurrence. The temporary intern map is dropped at `finish`; the complete outgoing arena is newly sorted and every reference is remapped.

```rust
fn combine(parts: Vec<CompactDfg>) -> Result<CompactDfg, &'static str> {
    let mut i = Intern::default();
    let mut edges = Vec::new();
    let mut defs: BTreeMap<BindingKey,Vec<OccId>> = BTreeMap::new();
    let mut uses: BTreeMap<BindingKey,Vec<OccId>> = BTreeMap::new();
    let mut labels = Vec::new(); let mut exact = Vec::new();
    let mut stats: BTreeMap<String,crate::cpg::RdFileStats> = BTreeMap::new();
    for p in parts {
        let mut live = vec![false;p.arena.rows.len()];
        let mut mark = |id: OccId| live[id.0 as usize] = true;
        for &(f,t) in &p.edges { mark(f); mark(t); }
        for ids in p.defs.values().chain(p.uses.values()) { for &id in ids { mark(id); } }
        for v in p.labels.values() { mark(v.from); mark(v.to); }
        for &(f,t) in p.exact_labels.keys() { mark(f); mark(t); }
        let mut r = vec![None;live.len()];
        for (old,row) in p.arena.rows.into_iter().enumerate() {
            if live[old] { r[old] = Some(i.put(row)?); }
        }
        let map = |id: OccId| r[id.0 as usize].expect("marked primary reference");
        edges.extend(p.edges.into_iter().map(|(f,t)| (map(f),map(t))));
        for (k,ids) in p.defs { defs.entry(k).or_default().extend(ids.into_iter().map(map)); }
        for (k,ids) in p.uses { uses.entry(k).or_default().extend(ids.into_iter().map(map)); }
        labels.extend(p.labels.into_values().map(|v| Label { from: map(v.from),to: map(v.to),confidence:v.confidence }));
        exact.extend(p.exact_labels.into_iter().map(|((f,t),confidence)| Label { from:map(f),to:map(t),confidence }));
        for (file,s) in p.rd_function_stats { stats.entry(file).or_default().merge(s); }
    }
    let (arena,r) = i.finish()?;
    for (f,t) in &mut edges { *f=r[f.0 as usize]; *t=r[t.0 as usize]; }
    remap_bindings(&mut defs,&r); remap_bindings(&mut uses,&r);
    let (mut forward,mut backward) = (BTreeMap::new(),BTreeMap::new());
    for &(f,t) in &edges { add_adj(&mut forward,&arena,f,t); add_adj(&mut backward,&arena,t,f); }
    let mut reduced = BTreeMap::new();
    for v in labels { add_label(&mut reduced,&arena,r[v.from.0 as usize],r[v.to.0 as usize],v.confidence); }
    let mut exact_labels = BTreeMap::new();
    for v in exact {
        exact_labels.entry((r[v.from.0 as usize],r[v.to.0 as usize]))
            .and_modify(|c: &mut FlowConfidence| *c=c.worst(v.confidence)).or_insert(v.confidence);
    }
    Ok(CompactDfg { arena,edges,defs,uses,forward,backward,labels:reduced,
        exact_labels,rd_function_stats:stats })
}
```

Add these update wrappers to `compact/update.rs`:

```rust
impl CompactDfg {
    fn empty() -> Self { combine(Vec::new()).expect("empty DFG") }
    fn remove_files(&mut self, excluded:&BTreeSet<String>) {
        let mut old=std::mem::replace(self,Self::empty());
        filter_primary(&mut old,excluded);
        *self=combine(vec![old]).expect("DFG exceeds u32 location capacity");
    }
    fn merge(&mut self,other:Self) {
        let old=std::mem::replace(self,Self::empty());
        *self=combine(vec![old,other]).expect("DFG exceeds u32 location capacity");
    }
}
```

Expose these wrappers to the facade/test module with the same crate-internal visibility as the compact views. Use `combine(vec![old,other])` for merge and `filter_primary` then `combine(vec![old])` for remove. Empty graph is `combine(Vec::new()).expect("empty DFG")`. The public API remains infallible as before; checked ID exhaustion uses `expect("DFG exceeds u32 location capacity")` at the build/update boundary, never wrapping. No published ID escapes, so reindexing cannot invalidate a caller's cached ID. `DataFlowGraph::clone` deep-clones compact storage for existing navigation refresh semantics; do not add an Arc mutation/COW policy in this lane.

Validate `combine` against legacy merge even for overlapping files, although the current producer expects disjoint file partitions. RD merging must call the existing `RdFileStats::merge` to union function identity sets. Unknown excluded files and empty exclusion still rebuild adjacency, replacing old independent representative rows from edge order exactly as legacy does.

- [ ] **Fixture measurement checkpoint before PR D.** For every fixture/generator emit old and compact accounted storage using the census conventions (capacity, owned string/path heaps, no BTree-node claim). Count all arena rows, not only edge endpoints; compare semantic occurrence/edge/label counts. The repeated-location synthetic fixture must strictly shrink; empty and all-distinct fixtures may grow and must be reported. This is a measured fixture accounting check, not extrapolated real-corpus acceptance. Preserve the record before codec work. **Task 7's controller measurement of PR E must pass before PR F starts.** No interim real-corpus run or old-cache conversion candidate is required.

- [ ] **Gate and commit.** GATE, update oracle after every intermediate operation, subset/merge corpus cases, targeted mutgate entries for wrong edge-end filtering, stale adjacency, overwrite merge, omitted worst merge, retained dead locations and altered stats. Controller title **`feat: preserve DFG update semantics with compact IDs`**, both trailers. Refresh measurement receipt/handoff before next PR.

### Task 5 (PR D): Native compact wire codec and validation, test-only

**Files:** `compact/{wire,validate}.rs`, `tests/{wire,corruption}.rs`, `src/cpg_cache/dfg_codec_tests.rs`. Production `DataFlowGraph` still serializes the old layout; do not bump its cache version in this test-only PR.

**Interfaces:** `WireDfg` stores the arena once, both legacy rank arrays, ID sequences/maps and RD stats. Persist the rank arrays to avoid an extra O(unique locations × log unique locations) sort on a cache hit; validate them in linear time. `TryFrom<WireDfg> for CompactDfg` validates **all** referenced IDs/keys before constructing a public graph. `WireDfgRef` borrows the same fields in the same declaration order; bincode bytes must equal owned serialization.

- [ ] **RED — Native wire round-trip and mutation tests.** For every Task 1 corpus graph: build legacy and compact independently, `bincode::serialize`/`deserialize` compact, then rerun the entire storage/public query oracle. Include empty graphs, defs-only, uses-only, label-only, exact-only, empty value vectors, and the inconsistent independent-adjacency fixture from Task 4. Preserve primary state: do not require every label to have an edge or every defs value's file to equal its map key.

```rust
#[test]
fn compact_stream_and_owned_bytes_match() {
    let c = CompactDfg::from_parts(adversarial_parts()).unwrap();
    let borrowed = bincode::serialize(&WireDfgRef::from(&c)).unwrap();
    let owned = bincode::serialize(&WireDfg::from(c.clone())).unwrap();
    assert_eq!(borrowed,owned);
    let decoded: WireDfg = bincode::deserialize(&borrowed).unwrap();
    let decoded = CompactDfg::try_from(decoded).unwrap();
    assert_eq!(compact_observation(&c),compact_observation(&decoded));
}
#[test]
fn out_of_range_group_is_rejected() {
    let c = CompactDfg::from_parts(adversarial_parts()).unwrap();
    let mut wire = WireDfg::from(c);
    let first = wire.forward.first_mut().unwrap();
    first.0 = LegacyId(u32::MAX);
    assert!(CompactDfg::try_from(wire).is_err());
}
```

Add this distinct in-range case; the fixture has one forward key, so its mutation cannot be rejected merely for map ordering:

```rust
#[test]
fn in_range_wrong_group_is_rejected() {
    let c=CompactDfg::from_parts(adversarial_parts()).unwrap();
    let mut wire=WireDfg::from(c);
    assert_eq!(wire.forward.len(),1);
    let old=wire.forward[0].0;
    let wrong=(0..wire.legacy_reps.len()).map(|n|LegacyId(n as u32))
        .find(|id|*id!=old).unwrap();
    wire.forward[0].0=wrong;
    assert!(CompactDfg::try_from(wire).is_err());
}
```

Register one corruption case per row in the validator matrix below; clone valid wire, change exactly one field, assert a returned error rather than a panic. Truncate serialized bytes at every field boundary recorded by the test serializer; bincode must reject each truncation. No test reads a real corpus cache.

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features dfg-oracle --lib data_flow::tests::wire > .review/dfg-compaction/logs/t5-red.log 2>&1
cat .review/dfg-compaction/logs/t5-red.log
```
Expected: missing codec API initially; malformed wire accepted by an unchecked decoder becomes the behavioral RED. After GREEN all corpus round trips and corruption cases pass.

- [ ] **GREEN — Wire shapes.** Derive serde for `OccId`, `LegacyId`, `Adj`, `Label`; private wrapper fields remain u32. Use vectors for map wire rows to **detect duplicates/out-of-order keys**, instead of allowing serde's BTreeMap deserializer to silently replace duplicates. Declare both wire structs in exactly this order:

```rust
#[derive(serde::Serialize,serde::Deserialize)]
struct WireDfg {
    arena: Vec<VarLocation>,
    legacy_reps: Vec<OccId>,
    legacy_by_occ: Vec<LegacyId>,
    edges: Vec<(OccId,OccId)>,
    defs: Vec<(BindingKey,Vec<OccId>)>,
    uses: Vec<(BindingKey,Vec<OccId>)>,
    forward: Vec<(LegacyId,Adj)>,
    backward: Vec<(LegacyId,Adj)>,
    labels: Vec<((LegacyId,LegacyId),Label)>,
    exact_labels: Vec<((OccId,OccId),FlowConfidence)>,
    rd_function_stats: BTreeMap<String,crate::cpg::RdFileStats>,
}
```

For borrowing, use a `MapSeq<'a,K,V>(&'a BTreeMap<K,V>)` serializer, avoiding a second vector of keys/values:

```rust
use serde::ser::SerializeSeq;
struct MapSeq<'a,K,V>(&'a BTreeMap<K,V>);
impl<K: serde::Serialize,V: serde::Serialize> serde::Serialize for MapSeq<'_,K,V> {
    fn serialize<S: serde::Serializer>(&self,s:S)->Result<S::Ok,S::Error> {
        let mut seq=s.serialize_seq(Some(self.0.len()))?;
        for row in self.0 { seq.serialize_element(&row)?; }
        seq.end()
    }
}
#[derive(serde::Serialize)]
struct WireDfgRef<'a> {
    arena: &'a [VarLocation],
    legacy_reps: &'a [OccId],
    legacy_by_occ: &'a [LegacyId],
    edges: &'a [(OccId,OccId)],
    defs: MapSeq<'a,BindingKey,Vec<OccId>>,
    uses: MapSeq<'a,BindingKey,Vec<OccId>>,
    forward: MapSeq<'a,LegacyId,Adj>,
    backward: MapSeq<'a,LegacyId,Adj>,
    labels: MapSeq<'a,(LegacyId,LegacyId),Label>,
    exact_labels: MapSeq<'a,(OccId,OccId),FlowConfidence>,
    rd_function_stats: &'a BTreeMap<String,crate::cpg::RdFileStats>,
}
impl<'a> From<&'a CompactDfg> for WireDfgRef<'a> {
    fn from(g:&'a CompactDfg)->Self {
        Self { arena:&g.arena.rows,legacy_reps:&g.arena.legacy_reps,
            legacy_by_occ:&g.arena.legacy_by_occ,edges:&g.edges,defs:MapSeq(&g.defs),uses:MapSeq(&g.uses),
            forward:MapSeq(&g.forward),backward:MapSeq(&g.backward),labels:MapSeq(&g.labels),
            exact_labels:MapSeq(&g.exact_labels),rd_function_stats:&g.rd_function_stats }
    }
}
impl From<CompactDfg> for WireDfg {
    fn from(g:CompactDfg)->Self {
        Self { arena:g.arena.rows,legacy_reps:g.arena.legacy_reps,
            legacy_by_occ:g.arena.legacy_by_occ,edges:g.edges,defs:g.defs.into_iter().collect(),
            uses:g.uses.into_iter().collect(),forward:g.forward.into_iter().collect(),
            backward:g.backward.into_iter().collect(),labels:g.labels.into_iter().collect(),
            exact_labels:g.exact_labels.into_iter().collect(),rd_function_stats:g.rd_function_stats }
    }
}
```

- [ ] **GREEN — Validate before indexing.** This validation deliberately permits legacy synthetic primary states while enforcing compact reference integrity. No forward/backward reconstruction on decode: each serialized map's independent representative/sequence is primary compatibility data.

```rust
fn ordered<K: Ord,V>(rows:&[(K,V)]) -> bool {
    rows.windows(2).all(|w| w[0].0 < w[1].0)
}
impl TryFrom<WireDfg> for CompactDfg {
    type Error = &'static str;
    fn try_from(w:WireDfg)->Result<Self,Self::Error> {
        let arena=Arena::from_wire(w.arena,w.legacy_reps,w.legacy_by_occ)?;
        let valid=|id:OccId| (id.0 as usize)<arena.rows.len();
        if !ordered(&w.defs)||!ordered(&w.uses)||!ordered(&w.forward)||!ordered(&w.backward)
            ||!ordered(&w.labels)||!ordered(&w.exact_labels) { return Err("DFG map order/duplicate"); }
        if w.edges.iter().any(|&(f,t)| !valid(f)||!valid(t)) { return Err("DFG edge ID"); }
        if w.defs.iter().chain(&w.uses).any(|(_,ids)| ids.iter().any(|&id| !valid(id))) {
            return Err("DFG binding ID");
        }
        for (key,row) in w.forward.iter().chain(&w.backward) {
            if !valid(row.representative)||row.neighbors.iter().any(|&id| !valid(id)) {
                return Err("DFG adjacency ID");
            }
            if *key!=arena.group(row.representative) { return Err("DFG adjacency group"); }
        }
        for (key,row) in &w.labels {
            if !valid(row.from)||!valid(row.to) { return Err("DFG label ID"); }
            if *key!=(arena.group(row.from),arena.group(row.to)) { return Err("DFG label group"); }
        }
        if w.exact_labels.iter().any(|((f,t),_)| !valid(*f)||!valid(*t)) {
            return Err("DFG exact label ID");
        }
        Ok(Self { arena,edges:w.edges,defs:w.defs.into_iter().collect(),uses:w.uses.into_iter().collect(),
            forward:w.forward.into_iter().collect(),backward:w.backward.into_iter().collect(),
            labels:w.labels.into_iter().collect(),exact_labels:w.exact_labels.into_iter().collect(),
            rd_function_stats:w.rd_function_stats })
    }
}
impl serde::Serialize for CompactDfg {
    fn serialize<S:serde::Serializer>(&self,s:S)->Result<S::Ok,S::Error> {
        WireDfgRef::from(self).serialize(s)
    }
}
impl<'de> serde::Deserialize<'de> for CompactDfg {
    fn deserialize<D:serde::Deserializer<'de>>(d:D)->Result<Self,D::Error> {
        let wire=WireDfg::deserialize(d)?;
        Self::try_from(wire).map_err(serde::de::Error::custom)
    }
}
```

The complete linear rank validator belongs in `compact/arena.rs` beside `from_sorted` (the latter is the build-time constructor):

```rust
impl Arena {
    pub fn from_wire(rows:Vec<VarLocation>,legacy_reps:Vec<OccId>,
                     legacy_by_occ:Vec<LegacyId>) -> Result<Self,&'static str> {
        checked_id(rows.len())?;
        if rows.windows(2).any(|w| exact_cmp(&w[0],&w[1])!=Ordering::Less) {
            return Err("DFG arena is not strictly exact-sorted");
        }
        if legacy_by_occ.len()!=rows.len() { return Err("DFG rank length"); }
        if legacy_reps.iter().any(|id| id.0 as usize>=rows.len()) {
            return Err("DFG rank representative ID");
        }
        if legacy_reps.windows(2).any(|w|
            rows[w[0].0 as usize].cmp(&rows[w[1].0 as usize])!=Ordering::Less) {
            return Err("DFG legacy rank order");
        }
        for (i,g) in legacy_by_occ.iter().enumerate() {
            let rep=legacy_reps.get(g.0 as usize).ok_or("DFG legacy rank ID")?;
            if rows[i]!=rows[rep.0 as usize] { return Err("DFG legacy rank identity"); }
        }
        for (g,rep) in legacy_reps.iter().enumerate() {
            if legacy_by_occ[rep.0 as usize]!=LegacyId(g as u32) {
                return Err("DFG rank representative group");
            }
        }
        Ok(Self {rows,legacy_reps,legacy_by_occ})
    }
}
```

Add corruption tests for rank length, out-of-range representative, duplicate/reversed legacy reps, out-of-range group, and in-range wrong-group mapping. Use two same-legacy byte variants plus a different-kind row so “correct rank, different exact occurrence” remains a valid positive case. The two rank arrays add **derived ≤8*u bytes** to the wire, about 1.7 MiB for the measured Prometheus core, to avoid resorting on each hit; this is negligible beside the inherited 551.3 MiB DFG wire size, but the full wire size is measured in Task 7.

Import `serde::{Serialize,Deserialize}` for trait method resolution. Bincode's existing cache-size/decoder policy remains the outer allocation boundary; this task adds reference validation, not a new general-purpose untrusted-cache allocator. `checked_id(rows.len())` validates size before indexing. Validation errors become the existing cache-miss/rebuild result at the cache layer, never a served partially valid graph.

| Corruption test | Must reject |
|---|---|
| duplicate arena row; reverse two exact rows | non-strict exact order |
| `u32::MAX` in each edge end, defs/uses member, adjacency representative/neighbor, label end, exact-label end | bounds before dereference |
| valid occurrence ID paired with a wrong in-range LegacyId | group mismatch |
| duplicate/reversed keys in each sequence map | map order/duplicate |
| truncated wire, invalid confidence discriminant | deserialize error |
| valid empty maps/empty value vector/label-only graph | **accept**, preserve bytes of query answers |

- [ ] **Gate and commit.** GATE plus all corruption mutants, borrowed/owned stream equality and corpus round trips. Controller title **`test: stage validated compact DFG cache codec`**, both trailers. Runtime effect **derived 0 MiB**; old production cache remains v117 until activation.

### Task 6 (PR E): Activate compact construction, accessors and cache version atomically

**Files:** production facade/build child modules; `src/cpg_cache.rs`, `src/cpg_cache/stream.rs`, codec tests; migrated consumer modules only if a missed raw access is found (enumerate it before editing). No oracle removal yet.

**Interfaces:** production `DataFlowGraph { storage: CompactDfg }`, same constructors/public query signatures as Task 2. All view methods forward directly to storage. `Serialize`/`Deserialize` delegate directly to compact wire; the legacy format is rejected by version **before payload decode**.

- [ ] **RED — Default-build representation assertion.** Add an internal test `production_build_is_compact_before_parallel_collection` using a test-only file-result hook in `build/file.rs`. It receives the already compact per-file result and records `(arena_rows, edge_count, legacy_adjacency_rows)` in an ordered BTreeMap by file; the last number must be zero because the producer has never allocated legacy adjacency. Under `dfg-oracle`, assert each compact result observes identically to the independently built legacy file. Also assert the production graph has no legacy-storage enum variant/field; the final raw-consumer search rejects unguarded `LegacyDataFlowGraph` or `legacy_storage` references in production facade/cache code. Do not make memory savings depend on a runtime flag.

```rust
#[test]
fn production_codec_shares_one_location_per_exact_identity() {
    let files = fixture_python("def f(x):\n y=x+x\n return y\n");
    let g=DataFlowGraph::build(&files);
    let encoded=bincode::serialize(&g).unwrap();
    let copy:DataFlowGraph=bincode::deserialize(&encoded).unwrap();
    assert_eq!(current_storage(&g),current_storage(&copy));
    let exact: BTreeSet<_> = g.storage.arena.rows.iter()
        .map(ExactVarLocation::from_legacy).collect();
    assert_eq!(exact.len(),g.storage.arena.rows.len());
}
```

`current_storage` is Task 1's view-based observation. This test fails to compile before activation and must kill a “decode into legacy then convert” mutant through the codec seam/decode counter test below, not just the existence of a compact field.

- [ ] **GREEN — Producer boundary.** In the extracted per-file closure, keep the original parsing/classification code and `FileRefs` local state. Replace its final return of `FileRefs` with this conversion, **inside the Rayon map**, before `.collect()`:

```rust
CompactDfg::from_parts(Parts {
    edges, defs, uses, labels, exact_labels,
    rd_function_stats: BTreeMap::from([(file_path.clone(),rd_function_stats)]),
    adjacency: None,
}).expect("DFG exceeds u32 location capacity")
```

The `ordered_files` vector remains in BTreeMap file order (`src/data_flow.rs:536-540`). Declare `per_file_refs: Vec<CompactDfg>` and collect from the indexed parallel iterator, retaining that order. Replace the entire old aggregation/expanded adjacency block (`:1180-1238`) with:

```rust
DataFlowGraph {
    storage: combine(per_file_refs).expect("DFG exceeds u32 location capacity"),
}
```

Keep per-file partition assertions immediately before conversion, using the local `defs`, `uses`, `labels`, `exact_labels`. Query methods are Task 2's migrated bodies. `empty()` uses `combine(Vec::new())`; `build_subset()` still filters exactly as baseline; updates delegate to Task 4. No post-build call `CompactDfg::from_parts(full_legacy_graph)` is allowed in production. Per-file scratch buffers die on that worker before the compact part is collected; combine moves each arena row and retains only compact temporary references.

- [ ] **GREEN — Cache version and early rejection.** Change `CACHE_VERSION` **117 → 118** (or current main + 1 if 118 has been used), its assertion and version history comment. Keep `CpgCacheRef` and `SerializedCpgRef` field order; their DFG borrow now uses compact Serialize. A version check after `read_cache_file` is too late for a changed layout (`src/cpg_cache.rs:541-552`). Add this helper in `stream.rs`, and call it instead of both `bincode::deserialize(&data).ok()` and `bincode::deserialize(&map[..]).ok()`:

```rust
fn decode_current_cache(data: &[u8]) -> Option<CpgCache> {
    // CpgCache/CpgCacheRef start with a bincode fixed-width u32 version.
    // Pin this prefix convention with an owned+borrowed encoder test.
    let version: u32 = bincode::deserialize(data.get(..4)?).ok()?;
    if version != super::CACHE_VERSION { return None; }
    bincode::deserialize(data).ok()
}
```

Keep the higher-level version check as defense in depth. Add test-only `PAYLOAD_DECODES` increment immediately before the full deserialize. Tests feed (a) 0–3 bytes, (b) serialized 117 + intentionally malformed payload, (c) future version + malformed payload: all return `None`, count stays zero, on both buffered and mapped reader paths. Current-version corrupt IDs return cache miss without panic; a valid compact cache increments once and hits. Existing lock/mmap/publish behavior stays unchanged, including v2 filenames. No nav-sidecar bump is needed because its bytes/meaning are unchanged.

- [ ] **GREEN — End-to-end fixture controls before measurement.** Every Task 1 case runs legacy build, compact production build, compact cache hit, and incremental remove/re-add. Assert storage and every DFG query after each transition. For CPG compare raw `node_indices()` order and serialized node payloads; raw edge indices/endpoints/weights in insertion order; lookup maps/telemetry; exact occurrence and conflict refusal cases. `edge_dump()` sorting alone is insufficient.

For public outputs use the existing API/renderers, with the same sources/diff/settings and immutable context for each backend: `output::to_review_output`, `output::to_sarif`, `targets::project`, navigation output serializers, `EvidenceHop`/`EvidencePath` serde. Assert `serde_json::to_vec` bytes, not unordered semantic equality; do not strip Evidence spans or sort findings to make them match. Retain the existing MCP transcript golden test unblessed. Add these paired checks to the current output tests at their existing input-construction seam, so `SarifInputs` and targets metadata are exactly the existing fixtures instead of an invented test-only output schema.

Run the explicit downstream slice (in addition to GATE):

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 cargo test --features mcp --test output --test reasoning --test navigation --test cli --test mcp > .review/dfg-compaction/logs/t6-outputs.log 2>&1
cat .review/dfg-compaction/logs/t6-outputs.log
```
Expected: all selected targets run, no changed output goldens, zero failures; totals recorded. Before replacing production on the implementation branch, controller retains baseline `prism`/`prism-mcp` binaries. Replay the existing CLI/MCP transcript fixtures against base and candidate in the **same temporary fixture checkout/path**, sequentially, with separate cache directories and identical request IDs/settings. Compare response payload bytes after the established transport fixture's existing envelope handling; add no new normalizer. This independent executable control covers changes shared by both in-process oracle assembly paths. Corpus real-repo measurement is still controller-only.

- [ ] **Gate and commit.** GATE; codec version tests and early-rejection mutant; full default/mcp suites and Tier-A; oracle in all build modes. Controller title **`perf: activate compact DFG storage and cache v118`**, both trailers, updating the literal version in the title if needed. This PR is **not accepted** until Task 7. Guessed savings remain 400–650 / 90–180 MiB, not a measured result.

### Task 7 (PR E acceptance): Prepare probe binaries and controller measurement

**Files:** scratch instrumented checkouts/binaries and `.review/dfg-compaction/acceptance/`; `scripts/dfg_compaction/compare_acceptance.py`; acceptance readout and handoff. Instrumentation is not merged into production.

**Pre-registration — written before any run:** these exact thresholds/classifications apply to both the activation candidate and post-cleanup candidate. **At most two complete base/candidate sets**: the initial set and one follow-up if invalid/noisy. Do not keep rerunning until a pass.

| Reading | PASS | FAIL / INCONCLUSIVE |
|---|---|---|
| Warm admissibility | 2/2 warm reports for each corpus/build have `cache.cpg_hit=true`, `call_edges_loaded=true`, and corresponding probe meta `cpg_hit=true`; matching corpus commit/workload hash, toolchain/profile/allocator/environment; distinct base/candidate source/build IDs recorded; finite process counters | A missing/false flag, mismatch, timeout, zero cases, missing raw prime or failed save is **INVALID**. Fix only the probe/control and consume the single follow-up set. |
| Process live heap after cache hit (`after_index.heap_in_use_bytes`) | `min(base two) - max(candidate two) >= 300*2^20` on Prometheus; `>= 75*2^20` on slicing | Below either threshold is **FAIL** if admissible. Do not claim resident-memory success from accounted DFG bytes alone. Also report `after_index - after_load_repo` heap; it is a diagnostic against loader drift, not a replacement metric. |
| Index load interval | `median(candidate) <= median(base) + max(0.05*median(base), 100 ms)` on both corpora, and each set's `(max-min)/median <= 0.10` | High spread is **INCONCLUSIVE**, not permission to increase tolerance. At admissible ≤10% spread, exceeding the bound is **FAIL**. 5%/100ms is an owner policy allowance (**guessed**), not a statistically established noise band. |
| Cold-build peak | Candidate prime's `after_call_edges.peak_bytes <= base prime's` on **each corpus**; fresh run directory causes a cold cache-writing prime | Any higher peak is **FAIL**; contention/unstable controls can make the whole set **INCONCLUSIVE**, never a cold-peak pass. n=1 per binary is a screening bound, not a confidence interval; a second full set is available once. |
| Physical residency | Report RSS, physical footprint, compressed bytes and peak; no claimed equivalence to heap | A material resident-footprint increase despite heap improvement needs controller investigation; do not call the lane goal achieved solely from accounting. |
| Correctness | All fixture/output/full-suite/Tier-A/mutation gates green on exact production candidate | Any output difference rejects activation regardless of memory saving. |

Evidence for the conservative timing rule: historical Prometheus index load 13.6/13.8 s and slicing 3.2/1.8 s on a nonquiet host (`.review/FINDINGS.md:4,10-18`). Those readings do **not** estimate a quiet-host noise distribution. This plan refuses to use slicing's historic 1.4 s spread as allowable regression. Report raw intervals, medians and spread alongside the verdict.

- [ ] **RED — Test the acceptance classifier with synthetic receipts first.** Use four small arrays, not real corpus processes. Required cases: exact threshold passes; one candidate warm value just above threshold fails; median within limit but 30% spread is inconclusive; 1-byte cold peak increase fails; false cache hit is invalid; missing/nonfinite counter is invalid. Put the following code in `scripts/dfg_compaction/compare_acceptance.py`; its module-level assertions are a lightweight self-test for the numeric kernel:

```python
from statistics import median

def classify(base_heap, cand_heap, base_ms, cand_ms, base_peak, cand_peak, minimum):
    import math
    numbers = base_heap + cand_heap + base_ms + cand_ms + [base_peak, cand_peak]
    if (len(base_heap),len(cand_heap),len(base_ms),len(cand_ms)) != (2,2,2,2):
        return "INVALID"
    if any(type(v) not in (int,float) or not math.isfinite(v) or v < 0 for v in numbers):
        return "INVALID"
    if min(base_ms+cand_ms) <= 0: return "INVALID"
    if any((max(v)-min(v))/median(v) > .10 for v in (base_ms,cand_ms)):
        return "INCONCLUSIVE"
    if min(base_heap)-max(cand_heap) < minimum: return "FAIL"
    if median(cand_ms) > median(base_ms)+max(.05*median(base_ms),100): return "FAIL"
    if cand_peak > base_peak: return "FAIL"
    return "PASS"

assert classify([1000,1000],[700,700],[1000,1000],[1100,1100],2000,2000,300) == "PASS"
assert classify([1000,1000],[700,701],[1000,1000],[1000,1000],2000,2000,300) == "FAIL"
assert classify([1000,1000],[600,600],[1000,1000],[800,1200],2000,2000,300) == "INCONCLUSIVE"
assert classify([1000,1000],[600,600],[1000,1000],[1000,1000],2000,2001,300) == "FAIL"
assert classify([1000,1000],[600,float('nan')],[1000,1000],[1000,1000],2000,2000,300) == "INVALID"
print("acceptance classifier: 5 boundary assertions passed")
```

Use raw bytes and milliseconds in real evaluation. The cache/provenance validation is separate and precedes this kernel; the kernel cannot certify those facts. Run the code as `python3 scripts/dfg_compaction/compare_acceptance.py > .review/dfg-compaction/logs/t7-classifier.log 2>&1`, then `cat .review/dfg-compaction/logs/t7-classifier.log`. Expected: the explicit five-assertion completion line below; a failing assertion exits before that line. Add the cache-flag/missing-counter cases to the artifact reader tests separately. Mutate `min(base_heap)-max(cand_heap)` to mean difference and the single-high-candidate case must fail.

- [ ] **Prepare same-environment binaries.** Controller creates two isolated checkouts: base at lane source `8b2744b3154b3664ffacb23c42f042a67981967a`, candidate at reviewed Task 6 head. If rebased, use the common main before this lane and record it, not a historical binary. Apply only the instrumentation delta from `c3162f94^..c3162f94` to both; do not use that old probe commit itself as the source baseline. Inspect its 14-file diff and reconcile moved storage fields. `PRISM_PROBE_OUT` activates instrumentation. Base/candidate use identical probe timers/schema and the same counting conventions.

```bash
git diff c3162f94^ c3162f94 -- src > .review/dfg-compaction/probe-instrumentation.patch
git show c3162f94:src/probe/census.rs > .review/dfg-compaction/probe-census-reference.rs
```
Expected: scratch instrumentation-only patch/reference, no implementation checkout changed by these read commands.

Candidate census must account for arena vector capacity + owned row string/path heaps **once**, both rank arrays, edge ID capacity, all map key/value inline and nested ID vector capacities, representatives, labels/exact labels and RD stats. Do not retain an expanded legacy snapshot for census or serialize to a full buffer for size measurement; reuse `src/probe/census/sizer.rs`'s counting writer. Emit the old semantic occurrence count in a separately named logical section; report physical compact fields as `arena`, `legacy_reps`, `legacy_by_occ`, `edges`, `defs`, `uses`, `forward`, `backward`, `labels`, `exact_labels`, `rd_function_stats`. Update the summarizer to print arena/rank fields instead of silently omitting them. Sum of accounted fields must not exceed process live heap; if it does, census is INVALID. Process counters remain the acceptance authority.

Test instrumentation on the tiny fixture corpus before controller handoff. Capture source commit, dirty patch hash, binary SHA256, `rustc -Vv`, cargo profile, grammar fingerprint, cache versions and allocator for both. Instrumented binaries must not have `dfg-oracle` enabled. Build each checkout **sequentially**, from its root:

```bash
nice -n 19 env CARGO_BUILD_JOBS=4 cargo build --release --features mcp --bin prism-mcp > .review/dfg-compaction/logs/probe-build.log 2>&1
cat .review/dfg-compaction/logs/probe-build.log
shasum -a 256 target/release/prism-mcp
```
Expected: release `prism-mcp` built successfully with mcp only; complete log and immutable binary hash. Controller copies the two binaries to `~/prism-evidence/mcp-memory/dfg-compaction-2026-10-14/bin/{base,candidate}/prism-mcp` and the receipts beside them. Do not overwrite an earlier run's binary or raw evidence.

- [ ] **Controller run — no implementation agent corpus process.** Read the retained procedure at `~/prism-evidence/mcp-memory/optimization-2026-10-14/PROCEDURE-probe.md`, driver `scripts/memreport/README.md:66-67,88-90,113-115`, summarizer and `run-probe.sh` there. Unlike that historic script, **do not proceed while the host is busy after a timeout**. Reserve the slot; record `pgrep -fl 'prism-mcp|cargo|rustc|nextest|mutgate|memreport.py'`, uptime, swap and RAYON_NUM_THREADS at every boundary. Expected: no other index/build/benchmark process; otherwise wait via controller scheduling. Do not kill unrelated work.

Freeze corpus commits and workload manifest hashes before the set. `RAYON_NUM_THREADS` is unset for both and recorded, as in the original probe. Use a fresh evidence directory `set-1`; driver output must not exist. Perform the commands in this fixed sequence: base Prometheus, candidate Prometheus, base slicing, candidate slicing. Each driver invocation runs one cold priming process plus two true cache-hit processes; never overlap them.

```bash
export DFG_EVIDENCE="$HOME/prism-evidence/mcp-memory/dfg-compaction-2026-10-14"
mkdir -p "$DFG_EVIDENCE/set-1"
PRISM_PROBE_OUT="$DFG_EVIDENCE/set-1/base-prometheus.jsonl" python3 scripts/memreport/memreport.py run --bin "$DFG_EVIDENCE/bin/base/prism-mcp" --corpus "prometheus=$HOME/code/bench-repos/prometheus" --workloads "$HOME/prism-evidence/mcp-memory/c1/workloads" --kinds warm --reps 2 --out "$DFG_EVIDENCE/set-1/base-prometheus" > "$DFG_EVIDENCE/set-1/base-prometheus.log" 2>&1
cat "$DFG_EVIDENCE/set-1/base-prometheus.log"
PRISM_PROBE_OUT="$DFG_EVIDENCE/set-1/candidate-prometheus.jsonl" python3 scripts/memreport/memreport.py run --bin "$DFG_EVIDENCE/bin/candidate/prism-mcp" --corpus "prometheus=$HOME/code/bench-repos/prometheus" --workloads "$HOME/prism-evidence/mcp-memory/c1/workloads" --kinds warm --reps 2 --out "$DFG_EVIDENCE/set-1/candidate-prometheus" > "$DFG_EVIDENCE/set-1/candidate-prometheus.log" 2>&1
cat "$DFG_EVIDENCE/set-1/candidate-prometheus.log"
PRISM_PROBE_OUT="$DFG_EVIDENCE/set-1/base-slicing.jsonl" python3 scripts/memreport/memreport.py run --bin "$DFG_EVIDENCE/bin/base/prism-mcp" --corpus "slicing=$HOME/code/prism-c1-corpus" --workloads "$HOME/prism-evidence/mcp-memory/c1/workloads" --kinds warm --reps 2 --out "$DFG_EVIDENCE/set-1/base-slicing" > "$DFG_EVIDENCE/set-1/base-slicing.log" 2>&1
cat "$DFG_EVIDENCE/set-1/base-slicing.log"
PRISM_PROBE_OUT="$DFG_EVIDENCE/set-1/candidate-slicing.jsonl" python3 scripts/memreport/memreport.py run --bin "$DFG_EVIDENCE/bin/candidate/prism-mcp" --corpus "slicing=$HOME/code/prism-c1-corpus" --workloads "$HOME/prism-evidence/mcp-memory/c1/workloads" --kinds warm --reps 2 --out "$DFG_EVIDENCE/set-1/candidate-slicing" > "$DFG_EVIDENCE/set-1/candidate-slicing.log" 2>&1
cat "$DFG_EVIDENCE/set-1/candidate-slicing.log"
```
Expected for **each** invocation: driver writes summary, raw `prime.json`, `warm-1.json`, `warm-2.json`, cache and sidecar; probe JSONL contains one miss then two hits grouped into three distinct probe PIDs in driver run order (prime, warm-1, warm-2). A zero exit code is insufficient; read the reports and cache-save diagnostics. The corpus source tree must remain unchanged. Warm runner's prime is a first build **with cache write**, not the separate no-cache `cold` kind; name that measurement accurately.

- [ ] **Read and classify the artifacts.** In raw files the report is wrapped under `raw['report']`, not at the top level (verified against existing receipts during planning). Import the checked-in `memreport_validate.validate` and require `validate('warm', report) is None`; require raw status success and probe/cache flags, then get stages by `stage['name']`. Live heap is `after_index.heap_in_use_bytes`; load ms is `after_index.elapsed_ms - after_load_repo.elapsed_ms` (`memreport_derive.py:15-21`). Cold prime peak is `after_call_edges.peak_bytes`. Require the prime to report both caches missed and both new cache files to exist. Run the updated summarizer:

```bash
python3 "$DFG_EVIDENCE/summarize_probe.py" "$DFG_EVIDENCE/set-1/base-prometheus.jsonl" "$DFG_EVIDENCE/set-1/candidate-prometheus.jsonl" "$DFG_EVIDENCE/set-1/base-slicing.jsonl" "$DFG_EVIDENCE/set-1/candidate-slicing.jsonl" > "$DFG_EVIDENCE/set-1/probe-summary.md"
cat "$DFG_EVIDENCE/set-1/probe-summary.md"
```
Expected: four complete summaries including physical arena/rank accounting, source/binary identity and all PIDs; no omitted candidate arena bytes. The summarizer is copied/adapted from the optimization evidence path, not assumed present magically.

Produce `acceptance.md` containing all raw metrics, formulas, base/candidate environment, cache hit evidence, correctness receipts and PASS/FAIL/INVALID/INCONCLUSIVE by corpus and criterion. Report lifetime peak, live heap, physical footprint, RSS and compression separately. If census accounting falls but process live heap does not, inspect a retained legacy copy/allocator effect; do not claim success. If process heap falls but load slows, isolate `load.read_decode`/rank reconstruction versus registry using matched stage CPU/wall timers; no guessed cause. Compare base in this same environment before attributing a regression.

One follow-up set, if needed, uses new `set-2` directories and reversed order within each corpus (candidate then base); repeat **both** corpora/binaries, not just a favorable failing sample. Both admissible sets must satisfy cold-peak and heap minima; combine load values only to report, not to erase a failed set. After the cap, report failure/inconclusive and hold PR E. If a targeted fix is made, it is a new reviewed candidate with a newly declared measurement round; controller decides scheduling, no silent retry.

- [ ] **Commit receipt and decision.** Controller title **`docs: record matched DFG compaction acceptance`**, both trailers. Include hashes/paths to durable raw evidence; do not commit multi-gigabyte caches/binaries. A PASS permits PR F to start. A FAIL/INCONCLUSIVE retains the reviewed implementation and receipts for bounded correction, never a fresh restart.

### Task 8 (PR F): Remove migration-only production scaffolding and verify final custody

**Files:** remove `legacy_storage.rs` from production/module declarations, any scratch activation switch, dead expanded serde adapters; keep frozen oracle/test modules and `dfg-oracle` feature; update architecture/release notes and handoff. No change to public query semantics or cache layout, so no second version bump unless bytes actually change.

- [ ] **RED — Prove the default/mcp release has no legacy route.** Add an architectural test that checks default/mcp production module reachability contains no oracle/legacy storage type, plus a compile check without `dfg-oracle`. Keep the feature-enabled dual representation tests: removing the test oracle is **not** this task. Mutant: reintroduce a legacy side field or production cache expansion; the storage assertion/physical census must detect it.

- [ ] **GREEN — Delete only dead adapters.** Production `DataFlowGraph` consists of compact storage, façade methods and producer/query modules. Test-only Parts import/export remains behind cfg gates for existing mutation tests; the native codec has no legacy branch. Verify direct-field inventory again and source-file sizes. The expected memory delta from E is **derived 0 MiB** because production already held only compact storage; an unexpectedly large improvement means E violated its design and the acceptance narrative needs correction.

```bash
rg -n 'LegacyDataFlowGraph|legacy_storage|dfg-oracle' src/data_flow.rs src/data_flow src/cpg_cache.rs src/cpg_cache > .review/dfg-compaction/logs/t8-legacy.txt
cat .review/dfg-compaction/logs/t8-legacy.txt
```
Expected: only test/explicit oracle-feature declarations and oracle implementation references; none reachable by default/mcp production. Inspect cfg context, not just text count.

- [ ] **Final gate.** Run GATE, authoritative **full** mutgate, Task 1 all-language oracle including cache/incremental transitions, unblessed output goldens and exact candidate identity. Controller reruns Task 7 on final binaries against the same-environment base with the same preregistration. This is a separately declared final verification set, not an extension of a failed Task 7 candidate. If cleanup is byte-identical production after symbol/module elimination, record that evidence but do not invent an unrun measurement receipt.

- [ ] **Commit and handoff.** Controller title **`refactor: remove legacy DFG production scaffolding`**, both trailers. Final handoff names every PR/commit, corpus/binary hashes, all test totals, mutations, Tier-A status and excluded checks. Say “memory acceptance passed” only when both corpora actually meet all thresholds; say “plan/implementation complete, measurement blocked” when the controller has not run it. No merge authority is implied by this plan.

## Paired output assertion seam for Task 6

Use the existing `ReviewInputs` fixture loader (its `build_warnings` field is private); do not hand-construct an incomplete substitute. The following helper accepts the two `api::ReviewRun` values produced by `run_review` with identical inputs/settings and each assembled CPG. It compares real renderer bytes and Evidence order, including source spans:

```rust
fn assert_output_pair(old:&prism::api::ReviewRun,new:&prism::api::ReviewRun,
                      inputs:&prism::api::ReviewInputs,meta:&prism::targets::TargetsMeta) {
    use prism::output::{to_review_output,to_sarif,SarifInputs};
    let json=|v:&serde_json::Value|serde_json::to_vec(v).unwrap();
    assert_eq!(old.results.len(),new.results.len());
    for (a,b) in old.results.iter().zip(&new.results) {
        assert_eq!(serde_json::to_vec(&to_review_output(a,&inputs.sources)).unwrap(),
                   serde_json::to_vec(&to_review_output(b,&inputs.sources)).unwrap());
    }
    let sarif_a=to_sarif(&SarifInputs::new(&old.findings).evidence(&old.evidence)
        .files(&inputs.files).sources(&inputs.sources).algorithms_run(&old.algorithms_run)
        .errors(&old.errors).parse_quality(&inputs.parse_quality));
    let sarif_b=to_sarif(&SarifInputs::new(&new.findings).evidence(&new.evidence)
        .files(&inputs.files).sources(&inputs.sources).algorithms_run(&new.algorithms_run)
        .errors(&new.errors).parse_quality(&inputs.parse_quality));
    assert_eq!(json(&sarif_a),json(&sarif_b));
    assert_eq!(serde_json::to_vec(&prism::targets::project(&old.findings,&old.evidence,inputs,meta)).unwrap(),
               serde_json::to_vec(&prism::targets::project(&new.findings,&new.evidence,inputs,meta)).unwrap());
    assert_eq!(serde_json::to_vec(&old.evidence).unwrap(),serde_json::to_vec(&new.evidence).unwrap());
}
fn assert_navigation_pair(old:&prism::navigation::Evidence,new:&prism::navigation::Evidence) {
    for format in ["json","text"] {
        assert_eq!(prism::output::navigation::render(old,format).as_bytes(),
                   prism::output::navigation::render(new,format).as_bytes());
    }
}
```

Invoke these helpers for the existing populated output/navigation fixtures and the Task 1 common-generator cases through the same fixture loader. The MCP transcript test compares the complete tool response using its existing fixed IDs and fixtures; its baseline is never regenerated by the candidate. Use the public `prism::api::{ReviewRun,ReviewInputs}` re-exports: the `run` and `review` modules are private on the inspected base (`src/api/mod.rs:3-17`).

Test-only CPG assembly entry, placed in the `CodePropertyGraph` impl owning `assemble_graph`:

```rust
#[cfg(any(test,feature="dfg-oracle"))]
pub fn assemble_with_dfg_for_compaction(
    cg:CallGraph,dfg:DataFlowGraph,files:&BTreeMap<String,ParsedFile>,
    type_db:Option<TypeDatabase>,
)->Self { Self::assemble_graph(cg,dfg,files,type_db) }
```

Build each context through the existing `CpgContext::build_with_cached_cpg(files,cpg,type_db)`; ordinary fixtures have no opted-in ephemeral owner. Run the detached-owner suite unchanged for its distinct authority behavior. Baseline executable transcript comparison remains required because this helper shares current assembly code.

## Planning validation (performed 2026-10-14)

**Source inspection:** clean initial tree; `pwd` matched the requested clone; `git rev-parse HEAD` printed `8b2744b3154b3664ffacb23c42f042a67981967a`. Read the current DFG, consumers, cache stream/version paths, common generators, languages, probe census at `scratch-opt-probe`, and retained procedure/raw report schema. The enumeration commands and source citations above are live inspections; the large-corpus sizes are inherited receipts.

**Small model only:** scratch directory `/private/tmp/dfg-compaction-plan-validation`. `types.rs` extracts unchanged AccessPath/VarLocation/ExactVarLocation/FlowEdge/ExactFlowEdge and FlowConfidence code from this source, stripping serde derives only. `arena.rs` and the insertion helpers in `model.rs` match the Task 3 kernel. The plan strengthens the originally symmetric three-neighbor test to four asymmetric neighbors so a reversal mutant is discriminated; that changed test body was not recompiled during planning.

```bash
nice -n 19 rustc --edition=2021 --test /private/tmp/dfg-compaction-plan-validation/model.rs -o /private/tmp/dfg-compaction-plan-validation/model-tests > /private/tmp/dfg-compaction-plan-validation/build.log 2>&1
cat /private/tmp/dfg-compaction-plan-validation/build.log
/private/tmp/dfg-compaction-plan-validation/model-tests > /private/tmp/dfg-compaction-plan-validation/tests.log 2>&1
cat /private/tmp/dfg-compaction-plan-validation/tests.log
```
Expected: compile a tiny standalone test executable, then 10 model tests pass. **Observed:** priority adjustment was refused (`nice: setpriority: Operation not permitted`), but rustc still compiled the tiny model, with four dead-code warnings. No priority success is claimed; no heavy build followed. Test output: **`10 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`**. This is admissible evidence for the model's identity/order/representative behavior, not host-priority compliance, source integration or memory performance.

**Derived arithmetic:** a Python calculation with the exact `n,u,h,hu` listed above produced `442.42284774780273` and `100.46309661865234` MiB estimates. Fixture extension inventory from `rg --files tests/fixtures` produced the 109-file/9-language counts in Task 1. No source parsing/build/query was performed on that inventory in this planning session. All 17 `make_*` generator return signatures were inspected: 11 normal triples and 6 security/stress pairs, matching the explicit arrays in Task 1.

**Acceptance arithmetic validation:** `python3 /private/tmp/dfg-compaction-plan-validation/acceptance-kernel.py > /private/tmp/dfg-compaction-plan-validation/acceptance.log 2>&1`, followed by `cat` of that log. Expected and observed: **`acceptance classifier: 5 boundary assertions passed`**. This validates only the numeric preregistration kernel; report/probe parsing and real measurements were not exercised.

**Static document checks:** 8 task headings; balanced code fences; required constraint/gate sections present; no unresolved generic task title or implementation placeholders. `rg -l '\bDataFlowGraph\b|\.dfg\b' src tests` returned **62 files**; broader DTO/taint/reasoning searches supplemented that census. These checks do not compile the plan's Rust snippets.

**Not checked:** full cargo suites, compilation of the complete production implementation/codec/update code, actual fixture dual runs, Tier-A, mutgate, runtime allocation/latency/peak, Linux, external library consumers, refresh peak, and correctness of a future probe port. Full corpus measurements are expressly reserved for the controller. Expected commands in this plan are instructions, not executed receipts.

## Owner questions and recommended defaults

1. **Acceptance allowance:** recommended pre-registration is ≥300 MiB Prometheus and ≥75 MiB slicing, ≤5% or 100 ms load increase with ≤10% within-set spread, and **no cold-prime peak increase**. These are concrete defaults; do not relax them after seeing results. Owner may choose stricter bars before the controller starts measurements.
2. **External direct-field Rust API consumers:** recommended default is the accessor source migration described in D7, preserving runtime outputs and query signatures. If unknown external consumers require field-level source compatibility, settle that before PR B; retaining duplicated public maps is incompatible with the intended memory result.
3. **Separate string/path interner:** recommended default is to defer it. Whole-location interning already shares repeated owned payload and has a measurable target; a second pool adds view/lifetime complexity for an unmeasured remaining population. Missing Task 7's threshold is a reason to reopen design, not automatically add scope.
4. **Oversized legacy modules:** recommended default is the declared extraction-only B1/B2 split to obey the <600-line source rule. No exception is assumed for new code or silently applied to existing edited modules.

No owner answer is needed to review this plan. Implementation, commits and real-corpus measurements are separate controller actions; this planning session writes the plan only and leaves it untracked.

## Executable test support used by Tasks 1–6

Place this support across `tests/observe.rs` and `tests/updates.rs` under `src/data_flow/`. Compile it under `cfg(any(test, feature="dfg-oracle"))`; expose only the corpus assertion entry point through the oracle feature. Import `super::{DataFlowGraph,VarLocation,VarAccessKind,FlowEdge,ExactVarLocation,ExactFlowEdge,AccessPath}`, the compact module's crate-internal types, `LegacyDataFlowGraph`, `ParsedFile`, `FlowConfidence`, `FlowDoubt`, and `BTreeMap/BTreeSet`. The compact module re-exports arena IDs within `data_flow` and uses `pub(super)` for test-observed fields/methods; it is not a public module. These helpers complete the named test seams; they are never in a production release.

```rust
macro_rules! view_storage {
    ($g:expr) => {{
        let g=$g;
        bytes(&(
            g.edges().iter().map(|e| e.to_owned()).collect::<Vec<_>>(),
            g.defs().map(|(k,v)| (k,v.to_vec())).collect::<Vec<_>>(),
            g.uses().map(|(k,v)| (k,v.to_vec())).collect::<Vec<_>>(),
            g.forward().map(|(k,v)| (k,v.to_vec())).collect::<Vec<_>>(),
            g.backward().map(|(k,v)| (k,v.to_vec())).collect::<Vec<_>>(),
            g.labels().map(|(e,c)| ((e.from,e.to),c)).collect::<Vec<_>>(),
            g.exact_labels().collect::<Vec<_>>(),
            g.rd_function_stats(),
        ))
    }};
}
fn compact_observation(g:&CompactDfg)->Vec<u8> { view_storage!(g) }
fn current_storage(g:&DataFlowGraph)->Vec<u8> { view_storage!(g) }
fn parts_from_legacy(g:LegacyDataFlowGraph)->Parts {
    Parts { edges:g.edges,defs:g.defs,uses:g.uses,labels:g.labels,
        exact_labels:g.exact_labels,rd_function_stats:g.rd_function_stats,
        adjacency:Some((g.forward,g.backward)) }
}
fn legacy_from_parts(p:Parts)->LegacyDataFlowGraph {
    let (forward,backward)=p.adjacency.unwrap_or_else(|| {
        let mut f:BTreeMap<VarLocation,Vec<VarLocation>>=BTreeMap::new();
        let mut b:BTreeMap<VarLocation,Vec<VarLocation>>=BTreeMap::new();
        for e in &p.edges {
            f.entry(e.from.clone()).or_default().push(e.to.clone());
            b.entry(e.to.clone()).or_default().push(e.from.clone());
        }
        (f,b)
    });
    LegacyDataFlowGraph {edges:p.edges,defs:p.defs,uses:p.uses,forward,backward,
        labels:p.labels,exact_labels:p.exact_labels,rd_function_stats:p.rd_function_stats}
}
fn pair(p:Parts)->(LegacyDataFlowGraph,CompactDfg) {
    (legacy_from_parts(p.clone()),CompactDfg::from_parts(p).unwrap())
}
fn built_pair(files:&BTreeMap<String,ParsedFile>)->(LegacyDataFlowGraph,CompactDfg) {
    let old=LegacyDataFlowGraph::build(files);
    let new=CompactDfg::from_parts(parts_from_legacy(LegacyDataFlowGraph::build(files))).unwrap();
    (old,new)
}
fn domain(g:&LegacyDataFlowGraph)->(Vec<VarLocation>,Vec<(String,usize)>) {
    let mut exact=BTreeSet::new();
    let mut add=|v:&VarLocation| {exact.insert(ExactVarLocation::from_legacy(v));};
    for e in &g.edges {add(&e.from);add(&e.to);}
    for vs in g.defs.values().chain(g.uses.values()) {for v in vs {add(v);}}
    for (k,vs) in g.forward.iter().chain(&g.backward) {add(k);for v in vs {add(v);}}
    for (f,t) in g.labels.keys() {add(f);add(t);}
    for e in g.exact_labels.keys() {add(&e.from.as_legacy());add(&e.to.as_legacy());}
    let original:Vec<_>=exact.into_iter().map(|v|v.as_legacy()).collect();
    let mut locations=original.clone();
    for v in &original {
        let mut shifted=v.clone();shifted.start_byte+=10000;shifted.end_byte+=10000;
        locations.push(shifted);
        let mut absent=v.clone();absent.file="absent.py".into();locations.push(absent);
        let mut path=v.clone();path.path=AccessPath::simple("__absent_compaction_path");locations.push(path);
        let mut function=v.clone();function.function_start_line+=10000;locations.push(function);
        let mut zero=v.clone();zero.line=0;locations.push(zero);
    }
    let mut lines:BTreeSet<_>=original.iter().map(|v|(v.file.clone(),v.line)).collect();
    let mut max:BTreeMap<String,usize>=BTreeMap::new();
    for v in &original {max.entry(v.file.clone()).and_modify(|n|*n=(*n).max(v.line)).or_insert(v.line);}
    for (file,line) in max {lines.insert((file,line+1));}
    lines.insert(("absent.py".into(),0));
    (locations,lines.into_iter().collect())
}
fn assert_all(old:&LegacyDataFlowGraph,new:&CompactDfg) {
    assert_eq!(old_storage(old),compact_observation(new));
    let (locations,lines)=domain(old);
    compare_queries!(old,new,&locations,&lines);
}
fn adversarial_parts()->Parts {
    let at=|file:&str,kind,start| {
        let mut v=loc("x",kind,start);v.file=file.into();v
    };
    let d0=at("a.py",VarAccessKind::Def,10);let d1=at("a.py",VarAccessKind::Def,20);
    let u0=at("a.py",VarAccessKind::Use,30);let u1=at("a.py",VarAccessKind::Use,40);
    let bd=at("b.py",VarAccessKind::Def,10);let bu=at("b.py",VarAccessKind::Use,30);
    let bx=at("b.py",VarAccessKind::Use,40);
    let key=|file:&str,name:&str|(file.into(),"f".into(),1,AccessPath::simple(name));
    let edges=vec![FlowEdge{from:d1.clone(),to:u1.clone()},FlowEdge{from:d0.clone(),to:u0.clone()},
        FlowEdge{from:d1.clone(),to:u1.clone()},FlowEdge{from:d1.clone(),to:bu.clone()}];
    let defs=BTreeMap::from([(key("a.py","x"),vec![d1.clone(),d0.clone()]),
        (key("b.py","empty"),vec![]),(key("b.py","foreign"),vec![d0.clone()])]);
    let uses=BTreeMap::from([(key("a.py","x"),vec![u1.clone(),u0.clone()])]);
    let mut labels=BTreeMap::from([((d0.clone(),u0.clone()),FlowConfidence::Exact)]);
    for kill_line in [7,3] {
        let c=FlowConfidence::NameOnly(FlowDoubt::Killed{kill_line});
        labels.entry((d1.clone(),u1.clone())).and_modify(|old|*old=old.worst(c)).or_insert(c);
    }
    labels.insert((bd.clone(),bu.clone()),FlowConfidence::Exact);
    let exact_labels=BTreeMap::from([(ExactFlowEdge::from_legacy(&FlowEdge{from:bd,to:bx}),FlowConfidence::Exact)]);
    let mut rd=crate::cpg::RdFileStats::default();rd.record_without_cfg("f".into(),1);
    let p=Parts{edges,defs,uses,labels,exact_labels,rd_function_stats:BTreeMap::from([("a.py".into(),rd)]),adjacency:None};
    let mut old=legacy_from_parts(p);
    let neighbors=old.forward.remove(&d0).unwrap();
    old.forward.insert(d0,neighbors);
    assert_eq!(old.forward.keys().next().unwrap().start_byte,10);
    let ((f,t),c)=old.labels.first_key_value().unwrap();
    assert_eq!((f.start_byte,t.start_byte),(10,30));
    assert_eq!(*c,FlowConfidence::NameOnly(FlowDoubt::Killed{kill_line:3}));
    parts_from_legacy(old)
}
#[test]
fn removal_uses_map_key_file_and_either_edge_endpoint() {
    let (mut old,mut new)=pair(adversarial_parts());
    let exclude=BTreeSet::from(["b.py".into()]);
    old.remove_files(&exclude);new.remove_files(&exclude);assert_all(&old,&new);
    assert!(new.edges().iter().all(|e|e.from.file!="b.py"&&e.to.file!="b.py"));
    assert!(new.defs().all(|(k,_)|k.0!="b.py"));
    assert!(new.labels().all(|(e,_)|e.from.file!="b.py"&&e.to.file!="b.py"));
    assert_eq!(new.exact_labels().count(),0);
    assert!(!new.edges().is_empty());
}
```

`built_pair` intentionally builds the legacy producer twice while the compact engine is test-only; it tests conversion and query independence, not a new producer. From Task 6 onward, add a **third** independently built production `DataFlowGraph::build(files)` and run `current_storage` and the same query macro against it. Do not replace the old oracle with a reconstruction from the compact graph.

For `compact::from_parts`/update tests, make the core structs' fields and methods visible to the parent data_flow test module (`pub(super)`) and import the private module's re-exports; keep the public façade as the only production external surface. No field/ID becomes externally public merely to accommodate tests.

## Literal task commit commands (controller only)

Stage the exact task-owned file list using `git add --` after inspecting the task diff; never `git add -A`. The titles below correspond one-to-one to the task commit steps. Each command has the same expected result: one reviewed task commit with both trailers; no amend, push or merge. If execution's cache version differs, change only the version in Task 6's title before committing.

```bash
git commit -m "test: freeze DFG behavior and fixture oracle" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01JhYB2xRuHRHxDFM7eDt8Fh"
```
Expected: Task 1 commit.

```bash
git commit -m "refactor: isolate ordered DFG accessors" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01JhYB2xRuHRHxDFM7eDt8Fh"
```
Expected: Task 2 commit (mechanical split commits use the same trailers).

```bash
git commit -m "feat: add compact DFG arena behind test oracle" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01JhYB2xRuHRHxDFM7eDt8Fh"
```
Expected: Task 3 commit.

```bash
git commit -m "feat: preserve DFG update semantics with compact IDs" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01JhYB2xRuHRHxDFM7eDt8Fh"
```
Expected: Task 4 commit.

```bash
git commit -m "test: stage validated compact DFG cache codec" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01JhYB2xRuHRHxDFM7eDt8Fh"
```
Expected: Task 5 commit.

```bash
git commit -m "perf: activate compact DFG storage and cache v118" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01JhYB2xRuHRHxDFM7eDt8Fh"
```
Expected: Task 6 commit.

```bash
git commit -m "docs: record matched DFG compaction acceptance" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01JhYB2xRuHRHxDFM7eDt8Fh"
```
Expected: Task 7 receipt commit.

```bash
git commit -m "refactor: remove legacy DFG production scaffolding" -m "Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_01JhYB2xRuHRHxDFM7eDt8Fh"
```
Expected: Task 8 commit.
