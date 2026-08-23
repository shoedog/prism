# p15a — Go provider reuse (roadmap #15a) — handoff

STATUS: DONE (code + docs) through fix wave 4. Wave 4 (targeted confirm):
c82ba91 (fix4: unconditional stash drop in build_with_cached_cpg), 9f2a396 +
c8c054b (fix4: all-constructors no-stash regression + synchronized
foreign-thread test). Sweep state: EVERY CpgContext constructor leaves the
stash None, asserted parameterized over all constructors. Commits: d81752f (WIP provider
reuse), 2672ddd (remove TEMP timing), 6f7d572 (fix1: reorder dispatch + registry
reuse), 587bef6 (fix1: WIP construction counter), 7cd9550 (fix2: WIP transfer
provider / incremental path — retention, incremental provenance boundary,
per-measurement test counter), 65256da (fix2: thread-local counter
attribution), 165c78a (fix3: drop stash in build_with_registry), 6adccfa+
f57a7d8 (fix3: deterministic cross-attribution regression).

## Fix wave 3 (review r3, converged round — same 2 WRONGs from both reviewers)

- WRONG (retention): `build_with_registry` neither transferred nor dropped the
  CPG build's stashed plain provider ⇒ long-lived context pinned a SECOND full
  Go dataset. FIXED: `take()` + drop; regression
  `p15a_fix3_build_with_registry_drops_stashed_go_provider` (RED confirmed by
  temporarily removing the drop).
- WRONG (handoff accounting): per-path table rows for SCOPED and DELTA/CPG-only
  were false; `build_with_registry` row missing. Table rewritten from the code
  (see above).
- SMELL→done: deterministic regression for the counter race —
  `p15a_fix3_foreign_thread_constructions_not_attributed` spawns a foreign
  thread constructing Go providers while a measurement is live and asserts
  zero cross-attribution (RED confirmed by temporarily restoring global
  generation attribution).
Final tree has no timing code; `cargo check` clean; `cargo test --lib p15a_fix1`
green. Gates + AFTER measurements: run by the controller (this lane's sandbox
cuts long commands).

## Construction accounting (corrected by review r1)

BEFORE (main): 4 plain/import-aware constructions per no-cache navigation build —
1. promotion-plain (`apply_go_embedding_promotion`),
2. dispatch-import-aware (`apply_go_interface_dispatch_with_scope_inputs`),
3. receivers-plain (receiver rematerialization),
4. registry-plain (`CpgContext::build_registry` via `from_built_cpg`).

AFTER fix wave 1: 2 — one dispatch-import-aware construction + ONE plain
construction shared by promotion, receiver rematerialization, and the type
registry. Peak live full `GoTypeData` extractions at any moment: 1.

## Baseline measurements (main + TEMP commit 0b4c29d, release build)

Per-construction wall time and total `prism nav --no-cache call-stats`.
Two datasets from different hosts/sessions — do NOT pool them:

Lane BEFORE dataset (this lane's sandbox, TEMP commit 0b4c29d):

etcd (3 clean runs):
| run | embedding-promotion | interface-dispatch | receiver-indices | total |
|-----|--------------------|--------------------|-----------------|-------|
| 1 | 7.42s | 7.27s | 7.12s | 36.43s |
| 2 | 7.11s | 7.27s | 7.28s | 36.24s |
| 3 | 7.23s | 7.10s | 6.95s | 35.82s |

prometheus (noisy host; six-run protocol — runs 2/3/4 rows are BLANK because
this lane's per-construction records for those runs were lost before the first
handoff draft; the earlier "best-effort selection" label was WRONG to present 3
of 6 as the dataset. GAP FLAGGED, not silently dropped):
| run | promotion | dispatch | receivers | total |
|-----|-----------|----------|-----------|-------|
| 1 | 7.70s | 7.78s | 8.66s | 51.37s |
| 2 | — | — | — | — |
| 3 | — | — | — | — |
| 4 | — | — | — | — |
| 5 | 8.16s | 8.36s | 10.39s | 58.48s |
| 6 | 9.20s | 9.10s | 8.18s | 57.20s |

Controller BEFORE dataset (TEMP-instrumented, controller host):
etcd totals 38.0 / 49.0 / 58.7 s; prometheus totals 63.7 / 53.0 / 54.3 s.
Controller AFTER (post-fix1): etcd totals 32.6 / 34.8 / 31.3 s; prometheus
45.6 / 42.9 / 47.0 s. Per-construction etcd 7.0/8.4/7.5 s → dispatch 7.5 s +
reused (promotion/receivers now share); prometheus 8.2/10.8/8.8 s → 8.8 s +
reused.

## Equivalence argument per site

- call_graph.rs ~2766 (`apply_go_embedding_promotion`, plain) and ~3246
  (`apply_go_receiver_indices` rematerialization, plain): both use
  `from_parsed_files` == `from_parsed_files_with_package_import_paths(files, &{})`
  (go.rs:285-287). IDENTICAL constructions → deduped: one provider is built per
  build sequence (`plain_go_provider_for_build`, lazily, only when Go files exist)
  and threaded via new pub(crate) `_with_provider` variants.
- call_graph.rs ~2899 (`apply_go_interface_dispatch_with_scope_inputs`,
  import-path-aware): NOT deduped. Proven import paths change each file's
  package-import input to `extract_from_file` (go.rs:~322-333), which changes
  the CANONICAL METHOD-SIGNATURE TYPE identity used by canonical satisfaction
  (type_providers/go.rs:~1138) — an unqualified local `T` can then compare
  exactly against an imported `pkg.T`. It does NOT change `GoOwnerIdentity`
  (which deliberately omits package/build partitions); the affected output is
  the dispatch table / satisfaction keys / declaration snapshots, so its
  extracted data genuinely differs from the plain one. Kept as a separate
  construction.

## What the threaded provider carries

`plain_go_provider_for_build(files)` returns an `Option<GoTypeProvider>` built once
per build sequence via `from_parsed_files` (import-path-free). It is threaded by
reference into `apply_go_embedding_promotion_with_provider` and
`apply_go_receiver_indices_with_provider`; both fall back to constructing their own
provider when passed `None` (non-Go builds never pay for construction). The
import-path-aware dispatch site keeps its own construction unchanged.

## Fix wave 2 (review r2)

Commit: 7cd9550. All three wave-2 items landed in one commit (the stalled
turn's uncommitted edits, reviewed then committed whole). Follow-up commit
65256da fixed a full-suite race in the test counter (thread-local attribution).

STATUS: fix wave 2 code committed through 65256da; `cargo fmt --check` clean;
`cargo test --lib` 717/717 green (twice); `cargo build --release` ok;
tier-a matrix 104/104 ok. Full gate + AFTER measurements: run by the
controller.

### Per-production-path accounting (corrected fix wave 3 — rewritten from the code; BEFORE fix1 = 4 constructions on the full path)

Counts are full GoTypeData CONSTRUCTIONS per no-cache Go build; peak live is
the max concurrently-alive extractions. Every path that builds a CPG pays
exactly one dispatch-import-aware construction (interface dispatch is never
deduped); the plain construction count depends on how many times a CPG build
or registry build runs in the path.

| production path (real entry point) | plain | import-aware | total | peak live |
|---|---|---|---|---|
| full build (`CpgContext::build` / `build_with_scope_graph_inputs`) — stash transferred to registry | 1 | 1 | 2 | 1 |
| prebuilt-registry (`CpgContext::build_with_registry`) — caller's registry used; stash DROPPED (fix3) | 1 | 1 | 2 | 1 |
| incremental rebuild (`NavigationIndex::build_incremental_from_previous` → `build_incremental_with_scope_graph_inputs` + `build_with_fresh_cpg`) — stash transferred | 1 | 1 | 2 | 1 |
| CLI partial-cache-hit (`run_review` fresh rebuild → `build_with_fresh_cpg`) — stash transferred | 1 | 1 | 2 | 1 |
| SCOPED (`CpgContext::build_scoped`) — filtered-subset CPG build (1 plain + 1 import-aware), filtered plain DROPPED, then FRESH plain for the registry over ALL files ⇒ 2 plain total | 2 | 1 | 3 | 1 |
| deserialized cache hit (`CpgContext::build_with_cached_cpg`) — BOTH inputs, universally true: (a) deserialized CPG → stash already None (serde skips the field), registry constructs fresh ⇒ 1 plain; (b) fresh-CPG input (e.g. `build_with_cached_cpg(files, CodePropertyGraph::build(files), None)`) → stash DROPPED unconditionally (fix4 — otherwise it survived alongside the fresh registry ⇒ 2 live datasets), registry constructs fresh ⇒ 1 plain | 1 | 0 | 1 | 1 |
| DELTA / CPG-only (`algorithms/delta_slice::slice` old-version graph via `build_enriched`) — 1 plain + 1 import-aware constructed, stash then dropped | 1 | 1 | 2 | 1 |
| AST-only (`CpgContext::without_cpg`) / registry-only callers | 1* | 0 | 1* | 1 |

\* only if Go files present and the registry build registers the Go provider;
`without_cpg` itself constructs nothing beyond its registry.

Wave-2 table corrections (found by review r3, both reviewers): SCOPED was
listed as 1 plain + 1 import-aware but is actually 2 plain + 1 import-aware
(the filtered CPG's own plain construction precedes the drop); DELTA/CPG-only
was listed as 0/0 but constructs 1 plain + 1 import-aware before dropping the
stash. `build_with_registry` was missing entirely.

Retention invariant: on EVERY path the CPG's `CallGraph` holds NO
`shared_plain_go_provider` after context construction — the provider is
`take()`-transferred into the registry (or dropped), never persisted on the
graph, so a long-lived navigation index / MCP session never pins the full Go
type data.

Provenance boundary: `build_with_fresh_cpg` (transfer) is used ONLY when the
CPG was provably rebuilt from the same current `files` map;
`build_with_cached_cpg` always constructs fresh because a deserialized cache
may predate file changes.

Tests (RED on pre-fix code):
- `call_graph::tests::p15a_fix2_pre_token_drop_does_not_underflow_live`
- `navigation::tests::p15a_fix2_incremental_rebuild_single_plain_extraction`
  (plain=1, peak live=1, no retained provider)
- fix1 test extended to assert `ctx.cpg.call_graph.shared_plain_go_provider.is_none()`

Test counter: global ARMED bit replaced by owned per-measurement
`MeasurementToken` with generation-keyed counters. Attribution is THREAD-LOCAL
(the token sets it on the acquiring thread; `build_pool::install` propagates it
onto the pool worker running the build op — cpg/build.rs routes both build
seams through that wrapper). A first draft used a process-global generation
and FLAKED in the full suite: a parallel un-tokenized test's Go construction
was attributed to the live token, inflating its plain count; thread-local
attribution removed the race (two consecutive full-suite runs green).
Drops are attributed to the generation the extraction was BUILT under, so
LIVE cannot underflow.

Import-path note (unchanged from fix1): proven package import paths change the
canonical METHOD-SIGNATURE TYPE identity used by dispatch satisfaction keys /
declaration snapshots; they do NOT change `GoOwnerIdentity`.

## Fix wave 1 (review r1)

Commits: 6f7d572, 587bef6.

- WRONG 1 (peak memory) — FIXED by REORDER + Arc retention: interface dispatch
  now runs BEFORE the shared plain provider is constructed, in both the full
  build (`CallGraph::build_with_receiver_config_and_scope_graph_inputs`) and the
  incremental path (`cpg/build.rs`). Promotion and dispatch are mutually
  independent — each reads only `files` (+ scope inputs for dispatch), writes
  disjoint CallGraph fields, and every downstream consumer runs after BOTH — so
  the swap is output-neutral (controller byte-identity gate verifies). Result:
  at most ONE full extraction alive at any moment.
- WRONG 2 (incomplete reuse) — FIXED: `CallGraph` carries the plain provider as
  a `#[serde(skip)]` field (`shared_plain_go_provider`, cache bytes unchanged)
  and `CpgContext::from_built_cpg` clones it (Arc-backed, no re-extraction) into
  `build_registry`. Only full-context builds (`build` /
  `build_with_scope_graph_inputs`) reuse it; scoped (`build_scoped`),
  cache-loaded (`build_with_cached_cpg`) and AST-only (`without_cpg`) contexts
  keep their OWN construction — their CPG may have been built from a different
  (stale or filtered) file set than the current `files`.
- SMELL 3 (counter + test) — DONE: test-only counters in go.rs
  (`test_counters`, armed/disarmed around the measured region; compiled out of
  non-test builds). Test
  `call_graph::tests::p15a_fix1_single_plain_provider_peak_live_one_through_full_context_path`
  asserts plain = 1, import-aware = 1, peak live = 1 through
  `CpgContext::build`. REDNESS confirmed empirically with two temporary
  uncommitted patches (each reverted immediately after):
  (a) passing `None` to `build_registry` (pre-fix registry behavior) →
  "expected exactly 1 plain construction" failed with left: 2;
  (b) restoring the pre-fix order (plain provider retained across dispatch) →
  "at most 1 full GoTypeData may be alive" failed with left: 2.
- Counter classification note: plain vs import-aware is decided by ENTRY POINT
  (`from_parsed_files` wrapper vs `_with_package_import_paths`), NOT by whether
  the paths map is empty — with no proven manifests the dispatch provider has an
  empty map but is still import-aware.

## Behavior that could differ

None expected. The deduped sites previously ran byte-for-byte the same constructor
on the same `files`, so reusing one instance is value-identical (`GoTypeProvider`
extraction is pure over `files`; no interior mutation between the two call points).
The kept-separate site is untouched.

## Pre-change controls generated

- interface-manifests (unchanged tree): were written to
  `target/p15a/manifest-before-{caddy,prometheus,etcd,hugo}.json` but target/ was
  subsequently cleaned — they are GONE. Controller must regenerate them from an
  UNCHANGED tree (separate worktree of 18b585a) before comparing against this branch.
- call-stats controls: scratchpad ctrl514-{ripgrep,caddy,prometheus,etcd,hugo}.txt
  (read-only).

## Gate results (this lane, at f270919, before TEMP removal)

- `cargo fmt --check`: clean.
- `cargo test`: 3290 passed / 0 failed / 1 ignored (3320-class, matches main).
- `cargo build --release`: ok.
- `eval/.venv/bin/tier-a --matrix-only --allow-stale-sut`: all rows ok (104/104),
  no failures.
- Five-corpus `prism nav --no-cache call-stats` vs ctrl514-*.txt:
  caddy / etcd / hugo / prometheus / ripgrep all RAW-BYTE IDENTICAL.
- Interface-manifest comparison: NOT done in-lane (pre-change manifests lost to
  the target/ clean, see above) — controller to regenerate + compare.
- AFTER measurements: not taken in-lane; controller to run the 3-run protocol on
  etcd + prometheus at 2672ddd (TEMP removed — per-construction timings will need
  the TEMP commit 0b4c29d cherry-picked or total wall time only).

Expected AFTER shape: promotion and receiver-indices share one construction → the
three per-construction timings should collapse to two (dispatch + one shared plain
construction); expected saving ≈ one plain construction ≈ ~7s of ~36s etcd (~19%)
and ~7–9s of ~55s prometheus (~13–16%).
