**MERGED PLAN REVIEW — S1 Perf Hardening + Level-4 Index Inversion Implementation Plan**

Both lenses reported successfully. Where they overlap (Task 9 parity gate, Task 3 scaffolding), severities are resolved below with rationale.

---

**BLOCKER 1 — Task 9 (Steps 9.1/9.2b/9.3): the spec-required serial↔parallel parity gate never becomes a real, automated test.**
Both lenses confirmed this independently. `cpg_build_is_identical_under_parallel_extraction` builds twice with the same implementation in one process, and `cache_blob_bytes_identical_serial_vs_parallel` compares two same-pool cached builds — both prove scheduling determinism, not serial==parallel. The promised "fixture-repo variant asserting equality between `RAYON_NUM_THREADS=1` and default" is not implementable as a single in-process test (the env var binds the global pool once), so it silently degrades to another same-pool run. The manual shell fallback is also broken: the tests write into `tempfile::tempdir()` directories that are deleted at test exit, so there is no artifact to compare across two cargo invocations, and Step 9.2b's shell line runs the target twice with no byte-comparison command between settings. Spec §2a calls byte-level cache parity "required for Slice C" and §7 row C gates on it; with no `CACHE_VERSION` bump, an ordering regression would ship silently into user caches.
*Severity resolution:* Executability rated BLOCKER, Coverage rated MAJOR — BLOCKER is right because a spec-**required** gate exists only as a manual step that, as written, cannot even be performed.
*Fix (Coverage's, the stronger one):* the "process-global pool" premise is wrong — `rayon::ThreadPoolBuilder::new().num_threads(1).build().unwrap().install(|| …)` routes `par_iter` inside the closure to a local single-thread pool. Build the serial reference in-process in both tests (CPG dump and cache blob) so the gate lives permanently in `cargo test`; drop the unimplementable fixture-variant description and the shell-matrix step.

**BLOCKER 2 — Task 8 (Steps 8.1/8.2): `load_repo_serial_reference` visibility makes the parity test a compile failure.**
The test lives in `tests/navigation/loader_test.rs`, which compiles as a separate crate importing through public `prism` API; a `pub(crate)` or `#[cfg(test)]` helper from `src/repo_loader.rs` is not callable there. The snippet also lacks imports for `LoadedRepo` and `load_repo_serial_reference` (current imports are only `load_repo` and `SkipReason`).
*Fix:* either move the parity test into a `#[cfg(test)]` unit module in `src/repo_loader.rs`, or make the helper public (non-`cfg(test)`) and add the explicit imports.

---

**MAJOR 1 — Task 3 (Step 3.1): the test snippet is not executable as pasted, and Step 3.2 lacks a STOP branch.**
`parse_fixture_files` does not exist in `src/cpg/tests.rs` — the only helper is a single-file `build_python_cpg`; neighboring tests construct `BTreeMap`s manually. The plan carries a fallback note for `has_dataflow_edge` but none for the file-map helper. Additionally, Step 3.2 expects the cross-file pin to "PASS already" with no instruction if it doesn't (contrast Step 5.2's explicit STOP) — if current resolution doesn't produce that arg→param edge, the executor stalls or improvises the parity baseline.
*Fix:* include the exact fixture-construction code (or rewrite to build the `BTreeMap` directly), and add a Step-5.2-style STOP branch ("if it fails, fix the TEST to match current behavior; the pin documents reality").

**MAJOR 2 — Task 3 (Step 3.1): the negative assertion targets the wrong line.**
The assertion checks `("m.py", 4, "q")`, but in the fixture `class K:` is line 4 and the nested `def f(q):` is line 5; DFG parameter defs are pinned to the function start line (`src/data_flow.rs:221-237`), so `q` would be at line 5. As written the negative assertion passes vacuously and would not catch binding to the nested parameter.
*Fix:* change to line 5, or derive the target from the actual param node location.

**MAJOR 3 — Task 9 (Step 9.2): C2 implementation guidance is under-specified for the actual loop structure, and the line anchors conflate three functions.**
`data_flow::build_from_refs` mutates shared `defs`, `uses`, and `edges` throughout one long nested pass (src/data_flow.rs:163-404); "local collections, then serial merge" needs a concrete per-file result type and exact merge order. In `call_graph.rs`, the plan's "Phase 1 (~line 67-100)" is actually `build_skeleton`'s Phase 1, and "Phase 2 (~line 145-190)" spans `CallGraph::build`'s start *and its Phase 1* before reaching the real Phase 2 (:187). A third per-file loop, `build_direct_subset` (:542), contains two of the spec's eight hot call sites and the plan is silent on whether it's in scope. Under subagent execution a fresh implementor can parallelize the wrong function.
*Fix:* correct the anchors to `build`'s :160/:187, state explicitly that `build_skeleton` and `build_direct_subset` stay serial, and specify the per-file result type plus the ordered flattening contract for each output vector/map.

---

**MINOR 1 — Task 9 (Step 9.1) ordering: make the Cargo `[[test]]` entry an explicit first action.**
The plan's file map does mention the `[[test]] name = "infra_parallel_equality"` entry, but Step 9.1 instructs writing and running the test before Step 9.2 without saying to register the target first — `cargo test --test infra_parallel_equality` fails until it exists. (Executability rated MAJOR, but since the entry is already in the file map this is an ordering clarification, not a missing requirement — MINOR.)

**MINOR 2 — Task 8 (Step 8.1): spec §5 MAJOR-5's classification order is asserted nowhere — the parity test is twin-vs-twin.**
Both `load_repo` and `load_repo_serial_reference` share the refactored walk, so a walk-order regression (e.g. extension check before read, flipping `bad.xyz` from `NotUtf8` to `Unsupported`) changes both twins identically and parity still passes; the NotUtf8 expectation lives only in a comment, and no existing test pins it.
*Fix:* add one absolute assertion that `skipped` contains `("bad.xyz", NotUtf8)`.

**MINOR 3 — Task 6: no test pins the full `CallSite` for the rewritten push loop.**
Spec §2.2 (r2-2) requires call-edge equality over `callee_name`, `line`, **and `qualifier`** "proven by differential tests"; Task 6.5's oracle proves index-contents equality only, and the existing Level-4 integration tests (`tests/integration/call_graph_test.rs:630-847`) assert callee-name membership only.
*Fix:* one pinned end-to-end assertion on a resolved Level-4 edge's full `CallSite` fields.

**MINOR 4 — Header: stale spec pointers.**
The plan cites "(rev 2.1)" and only the round-1 review record; the spec on disk is revision 3, which folds `s1-spec-review-r2-MCP-2026-06-11.md`. Content is current (r2 findings are cited throughout) — this is pointer rot, but an executor diffing plan-vs-spec will chase the wrong revision. Update both pointers.

**MINOR 5 — Task 10.2 drops the spec's designated warm-parity remedy.**
Spec §3 (r2-11) names lazy `OnceLock` construction as the fallback if the ≤10% warm-parity gate fails; the plan says only "explain if not." Record the fallback so a gate failure has a pre-agreed action.

**MINOR 6 — Task 7: the descope trigger keys on more than C2 needs.**
The assertion bundles `LoadedRepo: Send + Sync` with `ParsedFile`, and Step 7.3's descope fires on "the assertion STILL fails" — a `!Sync` field elsewhere in `LoadedRepo` would descope C2 spuriously. Risk is low (verified: `LoadedRepo` is `PathBuf` + `BTreeMap`s + `Vec` + `Option<TypeDatabase>` of `BTreeMap`s), but split into two assertions so the decision keys on `ParsedFile` alone.

**MINOR 7 — Missing task: CLAUDE.md Dependencies.**
Task 8 adds `rayon` as a runtime dependency; CLAUDE.md's Dependencies section enumerates the dep set and no task updates it. One line in Task 8 or 10.4.

---

**Retracted from earlier drafts (verified non-issues):** Task 9's `edge_dump`/`node()`/`node_indices()` API concerns (`node()`/`node_indices()` are already public; `edge_dump` is explicitly added in Step 9.2); Task 1's anonymous-function coverage (JS/TS queries include `arrow_function`/`function_expression` and `function_name()` returns `None` as the plan assumes); Step 5b comparand and the index emission-order argument both check out.

**Coverage confirmation:** spec §3 → Tasks 1-3, §4 B1 → Tasks 5-6, §6 → Task 4, §7 gates → 4.3/6.7/9.3/10, §9 → 10.5 all verified covered; decomposition and A→B1→C sequencing are sound.

**Verdict:** Not executable as-is — fix the two BLOCKERs (in-process serial-reference parity via `ThreadPoolBuilder::install()` in Task 9; helper visibility/imports in Task 8) and the three MAJORs (Task 3 fixture helper + STOP branch, Task 3 assertion line 4→5, Task 9.2 loop anchors and merge contract) before building; the MINORs are cheap pre-execution edits.