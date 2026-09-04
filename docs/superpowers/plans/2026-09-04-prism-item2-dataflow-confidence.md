# Prism Roadmap Item 2 (DataFlow Confidence via Reaching Definitions) Implementation Plan

> **v4 (2026-09-04) — sol r2 folded, spec v6.2.** Supersedes v3 (sol r1, spec v6), v2 (re-anchored to `bffb847`, spec v4) and v1 (written against `444b673`, spec v2/v3). Sol plan-review r2 (`~/code/tools/reviews/item2-plan-r2-sol.md`, FIX, W=2 S=1, converging — W3/W4 ADDRESSED, all three prior SOUND rulings preserved) is folded here; its two WRONGs are plan-only, and spec v6.2 carries the one spec-side change. See "Fix round 1" and "Fix round 2" at the top of *Rulings recorded*. Every `file:line` below was re-verified by opening it in `/Users/wesleyjinks/code/slicing-phase0` at `bffb84750d97f80bfdbeafa8a7cb58ea4f63b8fd`; the anchor-by-anchor record is `~/code/tools/grounding/item2-reanchor.md`.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Label every existing `CpgEdge::DataFlow` with conservative reaching-definitions confidence, carry the selected evidence into per-finding confidence, expose scoped/nominal reporting plus an emit-time confidence filter, and make the result observable and cache-safe — without changing the nominal DataFlow edge set or any byte-pinned legacy output.

**Architecture:** `DataFlowGraph` stays the source of DataFlow membership and gains a label map computed by an intraprocedural reaching-definitions pass over the existing line CFG. CPG Step 4 *derives* each `CpgEdge::DataFlow(FlowConfidence)` payload from that map, so graph and DFG cannot disagree; Step 5b (arg→param) derives its label from the already-resolved callee instead, because RD is intraprocedural. Algorithms carry an index-aligned, non-serialized `EvidencePath` per emitted finding, and every emitter calls one `classify_with_evidence` before applying the emit-time filter. `--resolution` changes reporting only: it never selects a different graph, cache artifact, or traversal.

**Tech Stack:** Rust 2021, petgraph, rayon, serde + bincode 1, clap 4 derive, serde_json, the Phase 0 `prism::api` facade, bash byte controls, Python Tier-A matrix via `uv`. No new crate dependencies.

**Spec:** `/Users/wesleyjinks/code/tools/specs/2026-09-04-prism-item2-dataflow-confidence-spec.md` — **v6.2 (2026-09-04), scheduled**. The spec is the binding authority; this plan argues from it and does not redesign it. All `§` references below are to that spec unless another document is named.

**Owner decisions:** `/Users/wesleyjinks/code/tools/DECISIONS.md` §A (A1), §B (B2–B8), §E (E4).
**Measurement pass:** `/Users/wesleyjinks/code/tools/logs/item2-census/REPORT.md` (92,338 functions, 14 corpora + 14 fixture packs).

---

## Custody — the exact base this plan is anchored to

| Fact | Value |
|---|---|
| Read-only analysis tree | `/Users/wesleyjinks/code/slicing-phase0` |
| Base commit | `bffb84750d97f80bfdbeafa8a7cb58ea4f63b8fd` |
| Base branch / PR | `phase0-sarif-targets-api` / PR #229 (pushed, **not merged**) |
| Implementation worktree | `/Users/wesleyjinks/code/slicing-item2` |
| Implementation branch | `item2-dataflow-confidence`, currently `80d9b5e` (parent `bffb847`) |
| Already on the branch | `examples/dfg_census.rs` only (the §12 Q1 measurement instrument, commit `80d9b5e`) |
| Detached worktree used to build the control binary | `/Users/wesleyjinks/code/slicing-phase0-review`, verified at `bffb847`, detached (no branch), clean |
| Same-environment control binary | **does not exist yet** — Task 0 builds it as `~/code/tools/bin/prism-base-bffb847` **from the detached worktree above**, never by checking out paths inside the item 2 worktree (sol r1 W4: that would delete `examples/dfg_census.rs`, which `80d9b5e` adds and Task 0b requires) |
| Last controller-run full-suite figure on this code | `3810 passed / 0 failed / 1 ignored` across 29 test binaries at `602a6ed` (`~/code/tools/logs/closeout/full-suite-final.log`), plus 2 doc-tests (`doctests-final.log`). Task 0 re-measures in this environment; that measurement, not this figure, is the gate-4 baseline |
| Stale artifacts that must NOT be reused as the item-2 control | `~/code/tools/bin/prism-base-c220525` and `~/code/tools/logs/baseline-c220525.log` (`3543 passed / 0 failed / 1 ignored`) — both predate Phase 0 |

---

## Global Constraints

Copied from the spec. Every task's requirements implicitly include this section.

### Doctrine

- **Nothing below Exact feeds an asserted finding** (`docs/superpowers/pipeline-lessons.md:138-139`; `docs/analysis/prism-post-plan-roadmap.md:165`). Spec §2.7 cites this as `CLAUDE.md:174-192`; that text is not in `CLAUDE.md` at `bffb847` — the doctrine's live home is pipeline-lessons.
- Every RD ambiguity resolves toward `FlowConfidence::NameOnly` — under-assertion, never over-assertion (§6). A downgrade to `CfgIncomplete` needs the same rigour as an Exact claim (owner B8): each such fixture names *which construct* made the timing unknown.
- **One cache transition per PR** (`docs/superpowers/pipeline-lessons.md:74-78`). `CACHE_VERSION` moves **55 → 56 exactly once** across the whole branch, with one appended history entry that fix waves amend rather than re-bump. `NAV_CALL_EDGE_CACHE_VERSION` stays **24** (`src/navigation/call_edge_cache.rs:64`, pinned `:398`); `SKIP_POLICY_VERSION` stays `2`.
- **Label-only in nominal mode** (§3 non-goal 1): add, remove, or re-endpoint **zero** DataFlow edges. RD labels the membership `DataFlowGraph::build_from_refs` already produces.
- `VarLocation::identity_key` stays line-granular `(file, function, function_start_line, line, path, kind)` (`src/data_flow.rs:32-42`); per-occurrence `DefId` identity is a future item (owner B8).
- `FlowEdge` (`src/data_flow.rs:88-91`) gains **no field**.

### Byte control (§5)

- `scripts/phase0-byte-control.sh` (213 lines at `bffb847`) is kept unweakened. `scripts/item2-byte-control.sh` **extends** it: it reuses that script's fixture enumeration (`:125`), `ALGOS` (`:153`), `FORMATS` (`:154`) and diagram case (`:185`), and adds `--format sarif` and `prism targets` comparisons under `--resolution nominal`.
- Both scripts compare **stdout + stderr + exit status** for every invocation. **Expected: zero differing invocations. Any difference is a STOP.**
- Taint is excluded (documented non-byte-stability, Phase 0 §8.5).
- Controls are **same-environment, same-base** (attribution control): the base binary and the candidate are built in the same worktree/toolchain. A base built elsewhere is not a control.
- **§5.2's "Phase-0 goldens" are not checked-in files.** No `*.sarif` golden and no targets golden exists in the tree; `tests/cli/sarif_test.rs` and `tests/cli/targets_test.rs` assert structurally and against `docs/contracts/targets.schema.json`. The SARIF/targets control therefore compares **same-base binary output captured at control time**.
- **Byte-control base re-basing (Task 0b):** Task 0b is a designated behaviour commit and *will* change bytes. From Task 0b onward the base binary for `scripts/item2-byte-control.sh` and `scripts/phase0-byte-control.sh` is **the Task 0b head**, built in the same worktree, recorded by sha256 in the handoff. Tasks 1–7 are zero-diff against *that* binary. The pre-Task-0b `~/code/tools/bin/prism-base-bffb847` is retained and is the control for Tasks 0 and 0b only.
- Per-binary cache-decision control (§5.3): each binary with its own empty `--cache-dir`, run/run/edit/run → `(created, unchanged, changed)` for both.
- Same-base `prism nav --no-cache call-stats` on the eleven `eval/corpora.toml` anchors — prism, ruff, ripgrep, caddy, cobra, prometheus, etcd, zap, black, httpx, mypy — leaf-by-leaf JSON diff with `dfg_labels` excluded and inspected separately. **A delta that removes Exact call edges is a STOP** (lesson 17, `docs/superpowers/pipeline-lessons.md:195-207`).
- Tier-A: `uv run tier-a --matrix-only --allow-stale-sut` immediately after `cargo build --release`, and `uv run tier-a --quick --allow-stale-sut` (`eval/README.md:69-83`). Do not rebaseline.

### Behaviour commits (§5, owner E4)

Byte control gates every **refactor and additive** commit on this branch. A *designated behaviour commit* may change pinned bytes only when all four hold: (a) Tier-A (`--matrix-only` plus the `--quick` LSP oracles) shows **no recall loss and precision at or above base**; (b) the affected goldens are re-blessed **in that same commit** with a diff review; (c) it is a **separate commit**, never mixed with refactor or perf work; (d) it stays inside the single 55→56 cache transition. **Item 2 has exactly one designated behaviour commit: Task 0b.** Flipping the default `--resolution` from `nominal` to `scoped` is a later, separately scheduled behaviour commit and is **not** in this plan.

### File and dependency limits

- **600-line cap** (`CLAUDE.md:267-271`): new and split source/test files stay below 600 lines. This binds `src/cpg/flow_confidence.rs`, `src/cpg/reaching.rs`, `tests/integration/dfg_label_parity_test.rs`, and `tests/cli/min_confidence_test.rs`. If one would reach 600, split it and amend the permitted-file list first.
- **No new crate dependencies.** `Cargo.toml [dependencies]` (bincode pinned `= "1"` at `Cargo.toml:41`) and `Cargo.lock` resolution are unchanged.
- Preserve `BTreeMap`/`BTreeSet` ordering for deterministic labels, evidence, JSON, cache bytes, and test dumps.

### Permitted files (§9)

**New:** `src/cpg/flow_confidence.rs`, `src/cpg/reaching.rs`, `tests/integration/dfg_label_parity_test.rs`, `tests/cli/min_confidence_test.rs`, `eval/fixtures/<lang>/dfg_reaching_*/**`, `scripts/item2-byte-control.sh`, `docs/superpowers/plans/2026-09-XX-prism-item2-*.md`, `docs/superpowers/handoffs/2026-09-XX-prism-item2-*.md`.

**Modified:** `src/cfg.rs` and the narrow lexical-arm metadata helper in `src/ast.rs` (Task 0 only), `src/cpg/types.rs`, `src/cpg/build.rs`, `src/cpg/query.rs`, `src/cpg/trace.rs`, `src/cpg/cfg_queries.rs`, `src/cpg.rs`, `src/data_flow.rs`, `src/cpg_cache.rs`, `src/finding_confidence.rs`, `src/algorithms/{provenance_slice,taint,chop,left_flow,full_flow,echo_slice,membrane_slice,gradient_slice,circular_slice}.rs`, `src/reasoning/shape.rs`, `src/navigation/queries.rs`, `src/cli.rs` and `src/main.rs`, `src/output/{sarif,review,review_compact}.rs`, `src/api/run.rs`, `src/slice.rs`, `src/cpg/tests.rs`, `src/cpg/multiline_call_arg_tests.rs`, `tests/{common/mod.rs,ast/cpg_test.rs,ast/dfg_test.rs,integration/core_test.rs,infra/parallel_equality_test.rs}`, `eval/tier_a/matrix.py`, `CLAUDE.md`, `README.md`.

**Forbidden:** `find_path_references_scoped`, `is_shadowed_at`, `scope_has_declaration` in `src/ast.rs`; `src/languages/**`; `src/resolution*.rs`; `src/call_graph.rs`; `src/name_resolution/**`; `src/navigation/call_edge_cache.rs`; `src/mcp/**`; `Cargo.toml` dependencies; `eval/corpora.toml`; `docs/eval/tier-a/*.json`. If compiled reality requires any other forbidden change, **stop and amend the design**.

> **Task-0b-only amendment to §9 (controller ruling, 2026-09-04).** Task 0b additionally permits `src/ast.rs` statement collection (`statements_in_function` `:5778`, `collect_statements` `:5805`, `collect_statement_spans` `:5826`, `collect_nested_statements` `:5849`) and the per-language statement/wrapper tables under `src/languages/**` (`is_statement_node` `:800`, `is_control_flow_node` `:576`). This is an explicit amendment, recorded here and to be folded into spec v5 — it is **not** a silent widening, and it applies to **Task 0b only**. `find_path_references_scoped`, `is_shadowed_at`, and `scope_has_declaration` remain forbidden in every task including 0b.

### RED shape (§10 "RED shape", sol r1 S1) — plan-wide

For **every new semantic contract**, the RED record has **two parts**, in this order:

1. **Feature-absence on the exact predecessor.** Run the new test against the predecessor state. It fails to compile because the symbol does not exist. **Record it as "feature absent" — it is not the RED** (§8 gate 3). A compile error proves only that a name is missing.
2. **Assertion-level RED against a conservative stub.** Add the smallest stub that *compiles* and is deliberately wrong in the conservative direction — a constructor that returns the most pessimistic value, a function that returns an empty set, a classifier that always answers `Unlabeled`. Run the test again. It must now fail **on an assertion**, naming the expected and actual values. That failure is the RED, and it is what proves the test discriminates the semantics rather than the symbol's existence.

Both observations go in the task report. Only then implement. **Task 7 (closeout) is exempt** — it asserts over already-built artifacts and has no new contract. Tasks 0, 1, 2, and 4 previously went straight from "does not compile" to GREEN; each now carries an explicit stub step.

### Review, convergence and commits

- Implementation review cap: **2 rounds**, declared before dispatch. At the cap, classify before acting — converging (fewer, smaller, non-repeating findings) ⇒ fold the bounded fixes and disclose the extension in one line; open-class (each round surfaces new instances of the same kind) ⇒ park the slice and escalate to spec §12. Never restart a partially reviewed artifact to escape convergence pressure.
- Every finding is tagged WRONG or SMELL; WRONG first; a finding without a concrete input/state and incorrect result is a SMELL, never a blocker.
- Clippy: `cargo clippy --all-targets --all-features -- -D warnings`. If the base is not clean, diff the complete normalized warning set against the base **built in the same worktree**; only new warnings block.
- Full suite: `cargo test --all-targets --all-features --no-fail-fast 2>&1 | tee /tmp/item2-suite-<task>.log`, totals via `awk` over **every** `test result:` line, never `tail`.
- Before each commit: the task's focused GREEN checks plus `cargo fmt --all -- --check`; stage only the paths listed in that task.
- Commit subjects use `feat(item2):` / `test(item2):` / `refactor(item2):`, imperative, ≤72 chars, and end with both trailers exactly:
  `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>`
  `Claude-Session: https://claude.ai/code/session_015U8HwBTAFzFzqJbbq82JBT`

---

## Rulings recorded

### Fix round 1 — sol plan-review r1 (2026-09-04), folded into this v3

Round 1 of a declared cap of 2. Source: `~/code/tools/reviews/item2-plan-r1-sol.md` (FIX, W=4 S=1 I=0). Sol's six rulings: **PartialHit (b) SOUND**, **`worst()`-on-insert SOUND**, **gate 9 scope SOUND**; **Task 0b metric, cap-operand coverage, and RED shape UNSOUND** → all three fixed below. Spec v6 carries the matching amendments (§7.6, §8 gates 4/9/11, §10 Task 0b row + "RED shape", §7.2 cap tests, §13).

| Item | Finding | Where it landed in v3 |
|---|---|---|
| **W1** — finding-producer manifest | Only `provenance`, `taint`, `membrane`, `echo` construct `SliceFinding`s at `bffb847`; the other twelve named algorithms emit none, and `EvidencePath` is index-aligned to `findings`. Requiring non-empty finding pairs from `leftflow`/`fullflow`/`chop` had no design, and minting findings for them would change nominal bytes (§3/§5). | **Task 4** Steps 2, 2b, 8, 9 and **Task 7** Step 12: end-to-end finding+evidence pairs for the **four** producers only; traversal-only algorithms get labeled-walk unit tests on `dfg_forward_reachable_labeled` / `backward_reachable_labeled` / chop's on-path fold, plus §7.3 parity. Coverage-table rows for §7.6 and gate 11 updated. |
| **W2** — Task 0b acceptance metric | The fixture-pack ≥90 % threshold is **already green before the change** (REPORT.md §1/§H: packs score 90.5–100 % while corpora score 22–89 %), so it proves nothing; "materially improved" had no pass/fail meaning. | **Task 0b** Step 6: corpus-level, per-§7.1-language ≥ 90 % on the same Tier-A anchor corpora the census measured, with a [75 %, 90 %) attributed-residual band and a hard STOP below 75 %. The fixture-pack threshold is deleted. Before/after table required. |
| **W3** — cap-operand coverage | A 4,097-statement fixture made of assignments also exceeds `RD_MAX_DEFS`, so an implementation missing the line check passes through the def check. | **Task 2** Interfaces (`Unavailable` carries a cap-specific reason) and Step 4: three decoupled fixtures — line cap with **zero Defs**, def cap within ≤ 4096 statement lines, long span with few of both. |
| **W4** — base-binary custody | `git checkout bffb847 -- .` inside the item 2 worktree stages the deletion of `examples/dfg_census.rs` (added by `80d9b5e`), which a path-limited `git add` does not unstage; `\|\| true` hid a failed control setup. | **Task 0** Step 2 and **Task 0b** Step 9: build both control binaries in the detached worktree `/Users/wesleyjinks/code/slicing-phase0-review`. No checkout inside the item 2 worktree, no `\|\| true`. |
| **S1** — RED shape | Tasks 0, 1, 2, 4 recorded only compile-time "feature absent"; only Task 2 reached an assertion-level RED. | Global Constraints → **"RED shape"**, plus an explicit conservative-stub step in **Tasks 0, 1, 2, 4**. Task 7 exempt. |

### Fix round 2 — sol plan-review r2 (2026-09-04), folded into this v4

Round 2 of the declared cap of 2, and the last. Source: `~/code/tools/reviews/item2-plan-r2-sol.md` (FIX, W=2 S=1 I=0). Sol classified the loop **converging** with closed enumerable fixes and no open questions: r1's W3 and W4 are **ADDRESSED**, W1/W2/S1 are **PARTIAL** with a named residual each, and all three prior SOUND rulings (PartialHit choice (b), duplicate-key `worst()`, gate 9 scope) are **preserved in v3**. Both new WRONGs are plan-only; spec v6.2 carries the single spec-side change (Task 0b's middle band).

| Item | Finding | Where it landed in v4 |
|---|---|---|
| **W1 (r2)** — stub REDs are not compilable | Each "conservative stub" must be **closed over every symbol its RED command exercises**, or the recorded observation is still a compile error, not an assertion failure. Task 0 called `build_cfg_edges_impl`, which does not exist — `src/cfg.rs:29` exposes `build_cfg_edges`. Task 1's stub omitted the `From<ResolutionConfidence>` impl its own test calls and the module registration. Task 4's stub omitted `backward_reachable_labeled` and the whole evidence-vector transport its integration/CLI commands touch. | **Task 0 Step 5b** rewritten against the real seam; **Task 1 Steps 4b/4c** split into two RED records (unit-test stub; parity stub) each with a symbol-closure table; **Task 2 Step 6** closure table added; **Task 4 Steps 3b/3c** split into RED (i) classification and RED (ii) transport, each with a closed stub and a symbol-closure table. Every stub now carries an explicit *symbols the RED command exercises → defined by the stub* list. |
| **W2 (r2)** — Task 4 Step 2b is vacuous | `hops.iter().all(...)` passes on an empty `hops`; "non-empty map" admits fabricated `Exact` labels despite a `NameOnly` hop. | **Task 4 Step 2b** rewritten on fixtures with **known** labels by construction — a mixed Exact + `NameOnly(Killed{4})` case and a pure-Exact case — asserting set equality with the unlabeled twin, non-empty hops with the exact `(from, to)` endpoints, and the exact worst-confidence value, for `dfg_forward_reachable_labeled`, `backward_reachable_labeled`, and chop's on-path fold. |
| **S1 (r2)** — middle band not decidable | Recording "accept-or-second-pass" satisfies the written condition without saying which ruling is a pass. | **Task 0b Step 6** pass rule: **only a recorded `ACCEPT` passes**; a recorded `SECOND PASS` means extend the wrapper fix, re-run the census, and re-apply the whole rule before Step 7. Matches spec v6.2 §10. |

**Residuals sol named but did not require fixing, recorded so they are not lost:** Task 2 and Task 4 each document individual assertions their conservative stub satisfies by accident (`zero_cfg_edge_function_returns_unavailable_for_the_no_cfg_reason`; `crossed_unlabeled_is_unlabeled_even_with_exact_hops`, `a_missing_artifact_none_fails_to_unlabeled_not_empty_exact`). That is deliberate and is *why* they are called out in the step text — a pessimistic stub necessarily satisfies pessimistic assertions. What matters is that **no whole contract or gate** can pass against a stub, and after this round each RED record names at least one assertion that fails on a value.

### Owner rulings

The v1 section "Controller Rulings Required Before Dispatch" is closed. Each ruling and where this plan applies it:

| Ruling | Decision | Where this plan applies it |
|---|---|---|
| **B1** (spec §12 Q1–Q6) | Covered by **A1**: measurement pass authorised; report landed at `~/code/tools/logs/item2-census/REPORT.md`. | Task 2 takes `RD_MAX_DEFS = 2048`, `RD_MAX_LINES = 4096` from REPORT.md §2.3 — no longer placeholders. |
| **B2** RD signatures | *Implementer proposes, controller approves before RED.* | Task 2 **Step 1** is a signature-proposal report; RED cannot start until the controller pastes the approved declarations into Task 2's Interfaces block. Only `FlowConfidence`/`FlowDoubt`, the `labels` key shape, and the two cap constants are pre-bound. |
| **B3** flag types | *clap value enums* (closed set; `--help` prints the values). | Task 5 declares `#[derive(ValueEnum)] MinConfidence` and `ResolutionMode`. Noted: this deliberately departs from `src/cli.rs`'s existing `String + value_parser = [...]` convention (`:64-70`, `:279`, `:283`). |
| **B4** re-anchor | *Controller re-anchors the plan before Task 1.* | This document is that re-anchor; evidence in `~/code/tools/grounding/item2-reanchor.md`. |
| **B5** PartialHit label survival | *Delegated to the plan's analysis pass; sol rules on the choice.* | Section "PartialHit label survival — analysis" below. Choice: **lifecycle-carried store, not recompute-after-merge.** Acceptance is the §7.5 cold / full-hit / partial-hit label-parity test (gate 10). |
| **B6** Task 3 acceptance | *Task 3 records staged RED; Task 6 owns matrix GREEN.* | Task 3 commits §7.1 fixtures + expected labels and records the matrix as **staged RED** (the `nav dfg-stats --edges` oracle does not exist yet); Task 3's semantic GREEN is its Rust integration tests. Task 6 turns the matrix GREEN. Consequence for Task 0: the branch-arm *fixture* half of Task 0's spec acceptance also moves to Task 3/6; Task 0 proves arm IDs in `src/cfg.rs` unit tests. |
| **B7** §7.6 provenance wording | *Confirmed: `all_defs_of` bridge ⇒ `crossed_unlabeled` ⇒ `unlabeled/candidate`; controller corrects §7.6 text.* Spec v4 §7.6 is already corrected and now specifies **three** provenance cases. | Task 4 implements exactly three provenance end-to-end cases: Unlabeled (bridge), NameOnly (verified relation + ≥1 NameOnly hop), Exact (verified relation, Exact hops only). |
| **B8** three provisional rulings | *(i) capture-read ⇒ `CfgIncomplete` — yes, **provisionally**; "every" is strong wording and a downgrade needs the same rigour as an Exact claim. (ii) `--min-confidence` limited to finding-bearing outputs — yes. (iii) per-occurrence `DefId` identity is a future item — yes.* | (i) Task 3's capture fixtures each name the construct that made timing unknown, and Task 6's `dfg_label_nameonly_cfg_incomplete` counter makes the rule's cost visible per corpus — the evidence a later refinement needs. (ii) Task 5 rejects the flag for `text`/`paper`/`mermaid`/`callers`. (iii) Task 3 marks every same-line collapsed group `NameOnly(SameLine)` and never consults byte order. |
| **E4** behaviour commits | *Byte control stays the refactor/additive gate; designated behaviour commits may change bytes when Tier-A shows no recall loss, goldens are re-blessed in the same commit, and the commit is isolated.* | Global Constraints → "Behaviour commits". **Task 0b is item 2's only designated behaviour commit**; all other tasks are zero-diff. |
| **Task 0b** (controller ruling, 2026-09-04) | *Add a CFG statement-universe completeness slice between Task 0 and Task 3, as a designated behaviour commit.* Driver: REPORT.md §H measures only **22–89 %** of existing DataFlow edges as CFG-admissible (Rust 32.9 %, TSX 22.5 %, Go 47.9 %, Python 77.1 %); §4.2 step 33 would send the rest to `NameOnly(CfgIncomplete)` before RD ever runs. | Task 0b. **Cost if wrong:** one extra behaviour commit to review. **Cost if skipped:** an item 2 whose Rust and TSX labels are overwhelmingly `CfgIncomplete` — a shipped mechanism that is nearly inert on two of the three largest language families (lesson 16). |

---

## PartialHit label survival — analysis (owner B5)

**Choice: (b) a lifecycle-carried label store — `DataFlowGraph.labels` (plus a per-file RD-availability counter map) retained, removed and merged exactly as `edges` already is. Not (a) recompute-after-merge.** A sol reviewer rules on this.

### The mechanism, read at `bffb847`

1. **The incremental seam.** On `CacheResult::PartialHit` (`src/cpg_cache.rs:381`, constructed `:510`, consumed `src/api/review.rs:230`) the build calls, at `src/cpg/build.rs:350-360`:
   `cached_cg.remove_files(changed_files)` (350) → `cached_dfg.remove_files(changed_files)` (351) → `DataFlowGraph::build_subset(files, changed_files)` (356) → `cached_cg.merge(fresh_cg)` (359) → `cached_dfg.merge(fresh_dfg)` (360).
   `build_subset` (`src/data_flow.rs:188`) constructs fresh state **only for `changed_files`**. Retained files are never re-walked. That skip is the entire point of PartialHit.

2. **What the DFG's lifecycle actually maintains.** `remove_files` (`src/data_flow.rs:139-152`) retains `edges` by `!exclude.contains(&e.from.file) && !exclude.contains(&e.to.file)`, prunes `defs`/`uses` by their file-keyed tuple (`:146-149`), then calls `rebuild_adjacency()`. `merge` (`:157-167`) extends `edges`/`defs`/`uses` then calls `rebuild_adjacency()`. `rebuild_adjacency` (`:169`) clears and recomputes `forward`/`backward` from `self.edges`. So the DFG has exactly two kinds of state: **primary, file-partitioned** (`edges`, `defs`, `uses`) and **derived from `edges`** (`forward`, `backward`). There is no third kind and no cross-file dependency in either.

3. **A DFG edge is intra-file by construction.** `build_from_refs` (`src/data_flow.rs:209`) walks one `ParsedFile` at a time; every `FlowEdge` is pushed inside that file's walk — def spans from `assignment_lvalue_spans_on_lines` (`src/ast.rs:3609`), refs from `find_path_references_scoped` on a `func_node` of the same file (`:5294`), alias twins from the same span set (`src/data_flow.rs:500-527`). The two-sided file test in `remove_files` is defensive, not evidence of cross-file edges.

4. **An RD label is a pure function of one file's bytes.** RD's inputs per §4.2 are: `statements_in_function` (`src/ast.rs:5778`) for the line universe; `cfg::build_cfg_edges(parsed)` (`src/cfg.rs:29`), which takes a single `&ParsedFile`; that function's `defs`; and `build_alias_map` (`src/data_flow.rs:583`), which is function-scoped with no line dimension. None of these reads another file.

5. **Retention criterion.** A file is retained precisely because its content hash matched (`CacheResult::PartialHit { changed_files }`). Retained ⇒ bytes unchanged ⇒ parse unchanged ⇒ by (4) the label is unchanged. This is a **mechanism-level proof of parity**, not an empirical observation.

6. **The one cross-file label is already outside the store.** Step 5b arg→param (`src/cpg/build.rs:1000`, guarded by the `R6MultiOwnerCandidate` exclusion at `:923`) is the only graph DataFlow edge not represented in `dfg.edges`, and §4.1/§4.2 derive its label from `resolved.confidence` at assemble time. `assemble_graph` (`src/cpg/build.rs:431`) runs **in full on every build including PartialHit** — the incremental path rebuilds CG/DFG and then assembles. So the cross-file axis is recomputed every time and needs no store at all.

### Why recompute-after-merge is rejected

- **It costs exactly the work PartialHit exists to skip.** Recomputing after the merged graph means re-running `statements_in_function` + `build_cfg_edges` + the fixpoint for every function in *every* file, retained ones included. REPORT.md §3 measures the whole DFG build at **1.1 s (ruff) / 2.8 s (prometheus) / 2.4 s (etcd) / 1.3 s (mypy)**; recompute-after-merge restores that on every incremental review run, on the interactive path at `src/api/review.rs:230`.
- **It creates a second consult.** The cold path would compute labels per file inside `build_from_refs`; the partial path would compute them again over a merged whole-program DFG. Two code paths producing one value is precisely the "two copies of a consult drifting" risk §11 risk 2 names, and it is the risk §7.4 exists to close.
- **It buys no correctness.** By fact (5) the recomputed value is provably equal to the retained value. Paying full DFG-build cost for a provably identical answer is not conservatism, it is waste.

### What (b) requires, concretely

- `DataFlowGraph.labels: BTreeMap<(VarLocation, VarLocation), FlowConfidence>` gains the **same** lifecycle as `edges`: `remove_files` retains a key only when neither endpoint's file is in `exclude`; `merge` extends; `empty` starts empty. It is **not** rebuilt in `rebuild_adjacency` — it is primary state, not derived state.
- The seven per-label counters of `DfgLabelStats` (§4.7) are **not stored**. They are counted at Step 4 / Step 5b from the merged label state on every assemble, so they cannot disagree with what actually shipped.
- The two function-level counters — `dfg_rd_functions_over_cap` and `dfg_rd_functions_without_cfg` — are **not** recoverable from `labels` (an over-cap function and an ordinary CFG gap both yield `CfgIncomplete`). They ride in the DFG as file-keyed primary state, mirroring `defs`/`uses`:

```rust
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RdFileStats {
    pub functions_over_cap: usize,
    pub functions_without_cfg: usize,
}
// on DataFlowGraph, alongside `labels`:
pub rd_function_stats: BTreeMap<String /* file */, RdFileStats>,
```

  `remove_files` drops entries whose key is in `exclude`; `merge` inserts fresh entries (keys are disjoint by file, so no summing is needed and a collision is a bug the Task 3 test catches).

### Residual risk, and the guard

Option (b) admits one failure mode option (a) does not: a `labels` key surviving after its edge was removed, or an edge merged without its label. It is caught, not argued away:

- `tests/ast/dfg_test.rs` asserts `labels` key-set equality with the unique `(from, to)` set of `edges` **three times**: after a cold build, after `remove_files`, and after `merge`.
- A `labels` lookup for a synthetic `FlowEdge` returns `None`, never a default.
- Step 4 falls back to `NameOnly(CfgIncomplete)` on a missing label and increments visible telemetry (§6 rule 10) — absence is never silently `Exact`.
- **Acceptance: the §7.5 cold / full-hit / partial-hit label-parity test (gate 10).** Cold build, full `Hit`, then edit exactly one source file to force `PartialHit`; compare full labeled DFG membership and every `CpgEdge::DataFlow` payload against a **cold rebuild of the edited tree**, exercising retained, removed, and recomputed edges. Any missing, stale, or defaulted label is a STOP.

### One gap the spec leaves open (flagged, not worked around)

§7.4 fixes how the *parity test* collapses duplicate `(from, to)` observations (`.or_insert`). It does **not** state what `labels` does when two construction arms produce one key with different labels — the direct arm and the alias-resolved twin (`src/data_flow.rs:500-527`) can coincide. First-wins is order-dependent and could seat `Exact` where `NameOnly(AliasUnstable)` also applies. **This plan prescribes `FlowConfidence::worst()` on insert**, per §6's failure direction and §4.1's "must never contain `Exact` for one occurrence". Flagged for the sol seat; spec v5 should say so explicitly.

---

### Task 0: CFG branch-arm provenance, plus the same-environment control baseline

**Files:**

- Modify: `src/cfg.rs` (internal lexical-arm provenance at the sequential-edge seam, `:52-66`), and the narrow lexical-arm metadata helper in `src/ast.rs` (§9 permits this for Task 0 only; `find_path_references_scoped`, `is_shadowed_at`, `scope_has_declaration` remain untouched).
- Test: `src/cfg.rs` in-module `#[cfg(test)] mod tests` (the file already has one — `test_goto_edges` at `:797`).
- Create (outside the repo): `~/code/tools/bin/prism-base-bffb847`, `~/code/tools/logs/baseline-bffb847.log`.

**Interfaces:**

- Consumes: `cfg::build_cfg_edges(&ParsedFile) -> Vec<CfgEdge>` (`src/cfg.rs:29`), `build_function_cfg` (`:38`), `ParsedFile::statements_in_function` (`src/ast.rs:5778`), `Language::is_terminator` (`src/languages/mod.rs:858`).
- Produces: **internal** lexical-arm provenance consumed only by `src/cpg/reaching.rs` in Task 3. `CfgEdge { file, from_line, to_line }` (`src/cfg.rs:19-23`) keeps its public shape and its serialized bytes; **no line endpoint changes, no CPG edge is added or removed.** The provenance is an additional internal return channel (a sibling function or an extra out-parameter), never a new public field on `CfgEdge`.

**Steps:**

- [ ] **Step 1: Record the custody tuple.** In `/Users/wesleyjinks/code/slicing-item2`, run and record:

```bash
cd /Users/wesleyjinks/code/slicing-item2
git rev-parse HEAD && git branch --show-current && git status --porcelain
git merge-base HEAD bffb847
```

Expected: `HEAD` = `80d9b5e`, branch `item2-dataflow-confidence`, clean, merge-base `bffb847`.

- [ ] **Step 2: Build and pin the same-environment control binary — in the detached worktree, not here (sol r1 W4).**

`80d9b5e` **adds** the tracked file `examples/dfg_census.rs`. Restoring `.` from `bffb847` inside the item 2 worktree would stage that file's deletion, and Task 0's path-limited `git add` would not unstage it — the commit could delete the census instrument Task 0b depends on. Build the base in the worktree that is *already* at `bffb847` instead. No `|| true`: a failed control setup must stop the task, not be swallowed.

```bash
set -e
cd /Users/wesleyjinks/code/slicing-phase0-review
git rev-parse HEAD                       # must print bffb84750d97f80bfdbeafa8a7cb58ea4f63b8fd
test -z "$(git status --porcelain)"      # must be clean
cargo build --release
mkdir -p ~/code/tools/bin
cp target/release/prism ~/code/tools/bin/prism-base-bffb847
shasum -a 256 ~/code/tools/bin/prism-base-bffb847 | tee -a ~/code/tools/logs/item2-custody.txt
```

Then confirm the item 2 worktree still carries the instrument:

```bash
cd /Users/wesleyjinks/code/slicing-item2
git status --porcelain                   # must be empty
test -f examples/dfg_census.rs           # must exist
```

- [ ] **Step 3: Measure the base full-suite totals in this environment.** Spec §8 gate 4's `3543 passed / 0 failed / 1 ignored` is the **c220525** figure and is not the item-2 base. The last controller-run figure on `bffb847` code is `3810 / 0 / 1` across 29 binaries at `602a6ed` plus 2 doc-tests (`~/code/tools/logs/closeout/full-suite-final.log`, `doctests-final.log`) — treat that as the *expectation*, not the baseline; the baseline is what this run measures.

```bash
cd /Users/wesleyjinks/code/slicing-item2
cargo test --all-targets --all-features --no-fail-fast 2>&1 | tee ~/code/tools/logs/baseline-bffb847.log
awk '/^test result:/{p+=$4; f+=$6; i+=$8; n++} END{printf "result_lines=%d passed=%d failed=%d ignored=%d\n",n,p,f,i}' ~/code/tools/logs/baseline-bffb847.log
```

Record the totals. **These are the gate-4 baseline for every later task.** Expected: `failed=0` and totals at or near `3810 / 0 / 1` over 29 result lines. If `failed != 0`, report it and stop — a dirty base invalidates every attribution downstream. If the totals differ materially from 3810, say so in the task report before continuing; do not silently adopt a new number.

- [ ] **Step 3b: Measure Java and Lua DataFlow-edge populations (spec §8 gate 9).** Gate 9 records every non-`§7.1` language with its census DataFlow-edge count, and names Java and Lua as measured **in Task 0** — REPORT.md §G/§H has no row for either.

```bash
cd /Users/wesleyjinks/code/slicing-item2
cargo run --release --example dfg_census -- <a java tree> <a lua tree> \
  2>&1 | tee ~/code/tools/logs/item2-census/java-lua.log
```

Record, per language: function count, Σ `n_cfg_nodes`, Σ `n_dfg_edges`, Σ `n_dfg_edges_cfg_ok`. A language measuring **0 DataFlow edges** is `gate 9 N/A — no DataFlow`, exactly as Terraform is (0 edges, 7/7 functions with 0 CFG nodes). A language with edges is covered by the §7.3 parity test and its Exact/NameOnly poles are filed as a follow-up. **Never fabricate a pole.** Carry this table into Task 6 Step 10 and the handoff.

- [ ] **Step 4: Write the failing branch-arm unit test.** Add to `src/cfg.rs`'s test module. The diamond is C so `compound_statement` blocks are unambiguous:

```rust
#[test]
fn sequential_edges_carry_lexical_arm_provenance() {
    let source = r#"
int f(int c) {
    int x = 1;
    if (c) {
        x = 2;
    } else {
        x = 3;
    }
    return x;
}
"#;
    let parsed = ParsedFile::parse("d.c", source, Language::C).unwrap();
    let arms = crate::cfg::build_cfg_edges_with_arms(&parsed);

    // The edge set is unchanged: same count, same endpoints, same order.
    let plain = crate::cfg::build_cfg_edges(&parsed);
    assert_eq!(
        arms.iter().map(|(e, _)| e.clone()).collect::<Vec<_>>(),
        plain,
        "arm provenance must not change the edge set or its order"
    );

    // `x = 2` (line 5) and `x = 3` (line 7) are in DIFFERENT lexical arms.
    let arm_of = |line: usize| {
        arms.iter()
            .find(|(e, _)| e.from_line == line)
            .map(|(_, a)| *a)
            .unwrap_or_else(|| panic!("no CFG edge out of line {line}"))
    };
    assert_ne!(
        arm_of(5),
        arm_of(7),
        "then-arm and else-arm statements must not share an arm id"
    );

    // The globally-sorted sequential loop can emit `x = 2` -> `x = 3`, which
    // crosses arms. That edge must be *flagged*, not deleted.
    let cross = arms
        .iter()
        .find(|(e, _)| e.from_line == 5 && e.to_line == 7);
    if let Some((_, arm)) = cross {
        assert!(
            arm.crosses_lexical_arm(),
            "a 5->7 sequential edge crosses arms and must say so"
        );
    }
}
```

- [ ] **Step 5: Run RED part 1 — feature absence.**

Run: `cargo test --lib cfg::tests::sequential_edges_carry_lexical_arm_provenance`
Expected: compile error — `build_cfg_edges_with_arms` does not exist. Record as **feature absent**; per the RED-shape rule this is *not* the RED.

- [ ] **Step 5b: Run RED part 2 — assertion-level failure against a conservative stub.**

**Symbol closure (sol r2 W1).** The RED command `cargo test --lib cfg::tests::sequential_edges_carry_lexical_arm_provenance` exercises these symbols; every one must exist after the stub, or the observation is a compile error again:

| Symbol the test exercises | Provided by |
|---|---|
| `crate::cfg::build_cfg_edges_with_arms` | **stub** |
| `ArmProvenance` (+ `Debug`, `Clone`, `Copy`, `PartialEq`) | **stub** |
| `ArmProvenance::crosses_lexical_arm` | **stub** |
| `crate::cfg::build_cfg_edges` | **exists** — `src/cfg.rs:29`, `pub fn build_cfg_edges(parsed: &ParsedFile) -> Vec<CfgEdge>` |
| `CfgEdge` + `from_line` / `to_line` + `Debug`/`Clone`/`PartialEq` | **exists** — `src/cfg.rs:19-23`, derives `Debug, Clone, PartialEq, Eq, PartialOrd, Ord` |
| `ParsedFile::parse`, `Language::C` | **exists** |

Add the smallest stub that compiles and is deliberately wrong in the conservative direction — every statement in arm `0`, so nothing ever "crosses an arm". Note the call direction: in the **stub**, `build_cfg_edges_with_arms` wraps the real `build_cfg_edges`; in **Step 6** that inverts, and `build_cfg_edges` becomes the first component of `build_cfg_edges_with_arms` so there is one CFG walk.

```rust
/// STUB — replaced in Step 6.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ArmProvenance {
    pub(crate) from_arm: u32,
    pub(crate) to_arm: u32,
}

impl ArmProvenance {
    pub(crate) fn crosses_lexical_arm(self) -> bool {
        self.from_arm != self.to_arm
    }
}

/// STUB — every statement is arm 0, so `crosses_lexical_arm()` is never true.
/// Wraps the real `build_cfg_edges` (`src/cfg.rs:29`); Step 6 inverts this so
/// `build_cfg_edges` becomes this function's first component.
pub(crate) fn build_cfg_edges_with_arms(parsed: &ParsedFile) -> Vec<(CfgEdge, ArmProvenance)> {
    build_cfg_edges(parsed)
        .into_iter()
        .map(|e| (e, ArmProvenance { from_arm: 0, to_arm: 0 }))
        .collect()
}
```

Run the same command.
Expected: **assertion failure**, not a compile error — `assert_ne!(arm_of(5), arm_of(7))` fails with left `0`, right `0`, message "then-arm and else-arm statements must not share an arm id". Record this as the RED: it proves the test discriminates arm *semantics*, not the symbol's existence. The edge-set-equality assertion **passes** against the stub (the stub returns exactly `build_cfg_edges`' output in order), which is what proves the failure is the arm half and not an accidental change to the edge set.

- [ ] **Step 6: Implement the provenance channel.** Add a sibling entry point that returns the existing edges paired with arm provenance, and define `build_cfg_edges` as its first component so there is **one** CFG walk:

```rust
/// Lexical provenance for one CFG edge. Internal to the RD pass (item 2 Task 3):
/// `src/cfg.rs`'s sequential loop consumes the globally sorted, line-deduplicated
/// `statements_in_function` universe (`src/ast.rs:5778-5791`), so it can join two
/// statements that are in different lexical branch arms. RD must never call such
/// an edge a proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ArmProvenance {
    pub(crate) from_arm: u32,
    pub(crate) to_arm: u32,
}

impl ArmProvenance {
    pub(crate) fn crosses_lexical_arm(self) -> bool {
        self.from_arm != self.to_arm
    }
}

pub(crate) fn build_cfg_edges_with_arms(parsed: &ParsedFile) -> Vec<(CfgEdge, ArmProvenance)> { /* … */ }

pub fn build_cfg_edges(parsed: &ParsedFile) -> Vec<CfgEdge> {
    build_cfg_edges_with_arms(parsed)
        .into_iter()
        .map(|(edge, _)| edge)
        .collect()
}
```

Arm ids come from the lexical-arm metadata helper added to `src/ast.rs`: each statement line is tagged with the id of the innermost branch-arm block containing it (0 = function body). Do not change which lines `statements_in_function` returns — Task 0b owns that.

- [ ] **Step 7: Run GREEN.**

Run: `cargo test --lib cfg::` and `cargo test --lib ast::`
Expected: PASS, including every pre-existing `src/cfg.rs` test (`test_goto_edges`, `test_c_switch_fallthrough`).

- [ ] **Step 8: Prove zero behaviour change.**

```bash
cargo build --release
scripts/phase0-byte-control.sh ~/code/tools/bin/prism-base-bffb847 target/release/prism
```

Expected: zero differing invocations. Any difference is a STOP — Task 0 must not change bytes.

- [ ] **Step 9: Gates.** `cargo fmt --all -- --check`; clippy delta vs the same-worktree base.

- [ ] **Step 10: Commit.**

```bash
git add src/cfg.rs src/ast.rs
git commit -m "feat(item2): tag CFG sequential edges with lexical arm ids" \
  -m "Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>" \
  -m "Claude-Session: https://claude.ai/code/session_015U8HwBTAFzFzqJbbq82JBT"
```

---

### Task 0b: CFG statement-universe completeness — **designated behaviour commit**

> This is item 2's **only** commit permitted to change pinned bytes (§5 / owner E4). It is isolated: no refactor, no RD, no perf work, no cache bump. Its permitted-file amendment is stated in Global Constraints and applies to this task only.

**Why.** REPORT.md §H measures the fraction of existing DataFlow edges whose **use** line is in the CFG line universe and whose **def** line is in that universe or is the function start: **Rust 32.9 %, TSX 22.5 %, Go 47.9 %, Python 77.1 %, bash 38.2 %, javascript 45.2 %, typescript 46.4 %, C 57.8 %, C++ 64.1 %.** Spec §4.2 step 33 sends every non-admissible edge to `NameOnly(CfgIncomplete)` **before any cap is consulted**. Without this task, item 2 ships a mechanism that is ~2/3 inert on Rust and ~3/4 inert on TSX, which lesson 16 (`docs/superpowers/pipeline-lessons.md:187-193`) calls a presumed-broken mechanism. The fixture packs cannot surface this: every pack with any DFG edges scores **90.5–100 %**.

**The verified Rust mechanism** (REPORT.md §G, re-read at `bffb847`): `collect_statements` (`src/ast.rs:5805-5824`) pushes a child when `is_statement_node(kind)` and then recurses into its body **only if `is_control_flow_node(kind)`** (`:5816`). tree-sitter-rust wraps a block-tailed `if`/`match`/`loop` used as a statement in an `expression_statement`, which **is** a statement node (`src/languages/mod.rs:800-807`) but is **not** a control-flow node (`:576-610`) — so the walk records the header line and stops, and the entire body is invisible. Probe source is saved as `~/code/tools/logs/item2-census/probe-rust-cfg-coverage.rs`.

**Files:**

- Modify: `src/ast.rs` — `collect_statements` (`:5805`), `collect_statement_spans` (`:5826`), `collect_nested_statements` (`:5849`) only.
- Modify: `src/languages/mod.rs` — a new per-language wrapper-unwrap table only. Do not change `is_statement_node` (`:800`), `is_control_flow_node` (`:576`), `is_scope_block` (`:781`) or `is_declaration_node` (`:238`) semantics; add a table beside them.
- Test: `src/ast.rs` in-module tests; `src/cfg.rs` in-module tests.
- Re-bless in this same commit: whichever `tests/fixtures/**/golden` and structural expectations move (`tests/fixtures/nav_compat/golden`, `tests/fixtures/review_no_diagrams/golden` are the only checked-in goldens at `bffb847`).

**Interfaces:**

- Consumes: `Language::is_statement_node` (`src/languages/mod.rs:800`), `Language::is_control_flow_node` (`:576`), tree-sitter node kinds.
- Produces:

```rust
/// Kinds that WRAP a statement without being one semantically. When
/// `collect_statements` meets one it must descend through it, or the wrapped
/// body's lines never enter the CFG's line universe
/// (`ParsedFile::statements_in_function`). Measured: this is why Rust scores
/// 14.1% line coverage and 32.9% DataFlow-edge admissibility
/// (logs/item2-census/REPORT.md §G, §H).
pub fn statement_wrapper_kinds(&self) -> &'static [&'static str];
```

- `statements_in_function` keeps its signature `(&self, &Node) -> Vec<(usize, String)>` and its sort/dedup contract (`src/ast.rs:5788-5790`). It returns **more** lines than before. That is the behaviour change.

**Steps:**

- [ ] **Step 1: Write one RED probe per language, measured — not assumed.** REPORT.md §G measured the mechanism for **Rust only**; the other languages have measured *coverage* but not a measured *cause*. For each of Rust, TSX, TypeScript, JavaScript, Go, Python, C, C++, Java, Bash, Lua: dump the statement universe of a minimal function with a nested statement and record what is missing before proposing any fix. Reuse the census instrument's dump path:

```bash
cd /Users/wesleyjinks/code/slicing-item2
DFG_CENSUS_DUMP_STMTS=1 cargo run --release --example dfg_census -- <probe-dir> 2>&1 | tee /tmp/item2-stmt-probe-<lang>.log
```

Write down, per language, the expected observation if "wrapper kind blocks recursion" is true and what would falsify it. Record which you got. A language whose probe shows full coverage needs **no** change — do not edit it.

- [ ] **Step 2: Write the failing Rust unit test** in `src/ast.rs`'s test module:

```rust
#[test]
fn statements_in_function_descends_through_expression_statement_wrappers() {
    // tree-sitter-rust wraps a block-tailed `if` used as a statement in an
    // `expression_statement`, which is a statement node but not a control-flow
    // node — so `collect_statements` recorded line 3 and stopped, hiding 4 and 5.
    let source = "fn h(x: i32) {\n    let a = x + 1;\n    if a > 0 {\n        let c = a + 1;\n        g(c);\n    }\n    g(a);\n}\n";
    let parsed = ParsedFile::parse("m.rs", source, Language::Rust).unwrap();
    let func = parsed.functions().first().unwrap().node;
    let lines: Vec<usize> = parsed
        .statements_in_function(&func)
        .into_iter()
        .map(|(l, _)| l)
        .collect();
    assert!(lines.contains(&4), "if-body `let c` (line 4) must be a CFG statement line, got {lines:?}");
    assert!(lines.contains(&5), "if-body `g(c)` (line 5) must be a CFG statement line, got {lines:?}");
    assert!(lines.contains(&2) && lines.contains(&3) && lines.contains(&7), "pre-existing lines must survive: {lines:?}");
}
```

Add the analogous test for each language whose Step-1 probe showed a gap, using that language's own wrapper kind.

- [ ] **Step 3: Run RED.**

Run: `cargo test --lib ast::tests::statements_in_function_descends`
Expected: FAIL — `lines` is `[2, 3, 7]`; 4 and 5 are absent.

- [ ] **Step 4: Implement the minimal unwrap.** Add `statement_wrapper_kinds` to `src/languages/mod.rs` (Rust: `["expression_statement"]`; other languages only as their Step-1 probe justified) and consume it in `collect_statements` / `collect_statement_spans`:

```rust
if self.language.is_statement_node(kind) {
    out.push((line, kind.to_string()));
    if self.language.is_control_flow_node(kind) {
        self.collect_nested_statements(child, out);
    } else if self.language.statement_wrapper_kinds().contains(&kind) {
        // The wrapper's own line is already recorded above; descend so the
        // wrapped body's lines enter the CFG universe too.
        self.collect_nested_statements(child, out);
    }
}
```

Do not widen `is_statement_node` or `is_control_flow_node`: widening `is_control_flow_node` would also change `build_branch_edges` and the C switch-fallthrough behaviour that `test_c_switch_fallthrough` pins (`src/cfg.rs:797+`).

- [ ] **Step 5: Run GREEN on the unit layer.**

Run: `cargo test --lib ast::` and `cargo test --lib cfg::`
Expected: the new tests PASS and every pre-existing `cfg`/`ast` test still PASSes. A pre-existing CFG test that now fails is a real regression — fix the change, do not edit the test.

- [ ] **Step 6: Acceptance (i) — re-measure CFG admissibility at CORPUS level (sol r1 W2).**

The fixture packs already score **90.5–100 %** before any change (REPORT.md §1, §H), so a fixture-pack threshold is green before the work and proves nothing. The metric is **corpus-level, per §7.1 language, over the same Tier-A anchor corpora the census measured** — rerun with the identical command list from REPORT.md §5 so before and after are comparable:

```bash
cd /Users/wesleyjinks/code/slicing-item2 && cargo build --release
cargo run --release --example dfg_census -- --corpora ~/code/tools/logs/item2-census/corpora \
  2>&1 | tee ~/code/tools/logs/item2-census/after-0b.log
```

Produce this table in the task report — `after` is `Σ n_dfg_edges_cfg_ok / Σ n_dfg_edges` per language, corpus rows only, fixture packs excluded from the metric:

| §7.1 language | before (REPORT.md §H) | DFG edges measured | after | verdict |
|---|---:|---:|---:|---|
| rust | 32.9 % | 247,716 | _fill_ | ≥90 pass / [75,90) attributed / <75 STOP |
| go | 47.9 % | 1,312,976 | _fill_ | ″ |
| python | 77.1 % | 209,388 | _fill_ | ″ |
| typescript | 46.4 % | 2,330 | _fill_ | ″ |
| javascript | 45.2 % | 301 | _fill_ | ″ |
| *(tsx, reported alongside ts)* | 22.5 % | 4,396 | _fill_ | ″ |

**Pass rule, exactly:**
- **≥ 90 %** — pass.
- **[75 %, 90 %)** — **not a pass by default.** Two conditions must both hold before the controller rules at all: the task report attributes **≥ 95 % of that language's remaining non-admissible edges to named mechanisms**, each with a standalone probe in the style of REPORT.md §G (a minimal source, the dumped statement universe, and the tree-sitter node kind that blocked the walk); and the percentage is reported beside its function and edge counts. Then the controller records one of two rulings, and **only one of them passes** (sol r2 S1; spec v6.2 §10):
  - **`ACCEPT`** — recorded with a reason. This, and only this, is a pass.
  - **`SECOND PASS`** — extend the wrapper fix to cover the named mechanisms, **re-run Step 6's census, and re-apply this whole rule** to the new numbers before Step 7. A recorded `SECOND PASS` is *not* a pass; proceeding to Step 7 on one is a plan violation. Recording *a ruling* is the reporting obligation; recording `ACCEPT` is the pass condition, and v3 conflated the two.

  An unattributed residual is not eligible for either ruling — fix the attribution first.
- **< 75 %** — **STOP.** Report the residual mechanisms and escalate; do not raise the number by widening a predicate the Step-1 probe did not justify.

**Population caveat, stated so the [75,90) ruling has its denominator in view:** the eleven anchor corpora are 3 Rust + 5 Go + 3 Python. The JS/TS/TSX edges come from vendored files inside prometheus (387 tsx / 302 ts functions), ruff (223 tsx / 34 ts) and prism (2 tsx / 40 ts) — **javascript's whole corpus population is 36 functions / 301 edges**. Report the per-language function and edge counts next to the percentage; a corpus percentage over a three-figure edge population is a weak signal and the controller should rule on it as such rather than treating it as equivalent to Rust's 247k.

- [ ] **Step 7: Acceptance (ii) — Tier-A, the behaviour gate.**

```bash
cd /Users/wesleyjinks/code/slicing-item2 && cargo build --release
cd eval && uv run tier-a --matrix-only --allow-stale-sut
cd eval && uv run tier-a --quick --allow-stale-sut
```

Run the identical pair on `~/code/tools/bin/prism-base-bffb847` in the same environment. Expected: **no recall loss and precision at or above base.** A recall loss is a STOP; do not rebaseline `docs/eval/tier-a/*.json`.

- [ ] **Step 8: Acceptance (iii) — re-bless goldens in this commit.** Run the byte control to enumerate what moved:

```bash
scripts/phase0-byte-control.sh ~/code/tools/bin/prism-base-bffb847 target/release/prism 2>&1 | tee /tmp/item2-0b-bytes.log
```

Expected: a **non-empty, enumerated** set of differing invocations (this is the behaviour commit). For each, review the diff and record in the task report *why* the new bytes are correct (a statement line that was invisible is now in the CFG universe). Re-bless the affected goldens in this same commit. An invocation whose diff you cannot explain by the statement-universe change is a STOP.

- [ ] **Step 9: Acceptance (iv) — re-base the control, again from the detached worktree (sol r1 W4).** Commit Task 0b first (Step 12), then rebuild the control from `/Users/wesleyjinks/code/slicing-phase0-review` checked out at the Task 0b commit. Never check out paths inside the item 2 worktree, and no `|| true`:

```bash
set -e
cd /Users/wesleyjinks/code/slicing-item2
TASK0B=$(git rev-parse HEAD)
cd /Users/wesleyjinks/code/slicing-phase0-review
git fetch ../slicing-item2 "$TASK0B"          # the worktrees share the object store; fetch is a no-op if so
git checkout --detach "$TASK0B"
test -z "$(git status --porcelain)"
cargo build --release
cp target/release/prism ~/code/tools/bin/prism-item2-0b
shasum -a 256 ~/code/tools/bin/prism-item2-0b | tee -a ~/code/tools/logs/item2-custody.txt
git checkout --detach bffb847                  # leave the review worktree where Task 0 found it
```

**From here on `~/code/tools/bin/prism-item2-0b` is the byte-control base for Tasks 1–7**, and those tasks must be zero-diff against it. `~/code/tools/bin/prism-base-bffb847` is retained as the control for Tasks 0 and 0b only, and as the reference for the Task 0b diff review in the handoff.

- [ ] **Step 10: Acceptance (v) — no cache transition here.**

```bash
rg -n 'CACHE_VERSION' src/cpg_cache.rs   # still 55 at this point
rg -n 'NAV_CALL_EDGE_CACHE_VERSION' src/navigation/call_edge_cache.rs   # 24
```

The 55→56 bump belongs to Task 1 and happens exactly once.

- [ ] **Step 11: Full suite + gates.** `cargo test --all-targets --all-features --no-fail-fast` against the Task 0 baseline totals; `cargo fmt --all -- --check`; clippy delta.

- [ ] **Step 12: Commit — isolated.**

```bash
git add src/ast.rs src/languages/mod.rs tests/fixtures
git commit -m "feat(item2): descend through statement wrappers in the CFG universe" \
  -m "Designated behaviour commit (spec §5, owner E4): Tier-A no recall loss, goldens re-blessed in this commit, no cache transition." \
  -m "Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>" \
  -m "Claude-Session: https://claude.ai/code/session_015U8HwBTAFzFzqJbbq82JBT"
```

---

### Task 1: Flow-confidence payload, 35-site port, cache v56, label-insensitive parity

**Acceptance (spec §10 row 1):** §7.3 plumb-through parity GREEN; full suite at the Task 0 baseline totals; §5.1 byte control zero diffs against the Task 0b head.

**Files:**

- Create: `src/cpg/flow_confidence.rs` (< 600 lines including tests).
- Modify: `src/cpg.rs` (module + re-export, insert alphabetically at `:18-31`), `src/cpg/types.rs` (`:198`, `:313-315`), `src/cpg/build.rs` (`:578`, `:1000`, `:1325`, `:1461`), `src/cpg/query.rs` (`:239`, `:679`, `:745`, `:811`), `src/cpg/trace.rs` (`:852`), `src/algorithms/gradient_slice.rs` (`:110`), `src/algorithms/circular_slice.rs` (`:239`), `src/navigation/queries.rs` (`:2060`), `src/reasoning/shape.rs` (`:165`), `src/cpg_cache.rs` (`:161`, `:691`).
- Modify (compile-fix): `src/cpg/tests.rs` (14 sites), `src/cpg/multiline_call_arg_tests.rs` (3), `tests/common/mod.rs:134`, `tests/ast/cpg_test.rs:184`, `tests/integration/core_test.rs:873`, `tests/infra/parallel_equality_test.rs:121`.
- Create: `tests/integration/dfg_label_parity_test.rs`; modify `tests/integration/main.rs` to register it.
- Create: `scripts/item2-byte-control.sh`.

**Interfaces:**

- Consumes: `ResolutionConfidence` (`src/resolution.rs:26-29`), `ResolutionKind::R6MultiOwnerCandidate` (`src/cpg/build.rs:923`), `CodePropertyGraph::assemble_graph` (`src/cpg/build.rs:431`), `collect_step5b_edges` (`:878`), `step5b_edges_for_caller` (`:905`), `collect_step5b_edges_reference` (`:1367`), `argument_var_node_in_span` (`:1014`).
- Produces exactly (§4.1):

```rust
/// Confidence that a DataFlow edge's definition actually reaches its use.
/// Two-valued lattice (Exact ⊐ NameOnly), same shape as `ResolutionConfidence`,
/// but a DIFFERENT producer: the reaching-definitions pass, not the R1–R7 call
/// ladder. Loop-carried edges are Exact — RD proved reachability through a back
/// edge; the distinction is telemetry only (§4.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FlowConfidence {
    Exact,
    NameOnly(FlowDoubt),
}

/// Why an edge could not be proven. Every variant is a reason to UNDER-assert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FlowDoubt {
    /// RD proved a redefinition of the same path kills this def before the use.
    /// `kill_line` is the lowest-numbered killing statement line on any path.
    Killed { kill_line: u32 },
    /// Two Defs of one AccessPath collapse onto one line-granular endpoint (§4.3).
    SameLine,
    /// No CFG node for the def or use line, function over the RD cap, or the
    /// function has no CFG edges at all.
    CfgIncomplete,
    /// The edge exists only through the alias map.
    AliasUnstable,
    /// Step-5b arg→param edge whose resolved callee is NameOnly.
    CallNameOnly,
}

impl FlowConfidence {
    /// The lattice meet. NEVER use `std::cmp::min`: derived `Ord` puts `Exact`
    /// FIRST, so `min` returns the BEST label, not the worst. Same trap as
    /// `ParseQuality::min_over`, which uses `worst.max(..)`
    /// (`src/finding_confidence.rs:107`).
    pub fn worst(self, other: Self) -> Self;
    pub fn is_exact(self) -> bool;
    pub fn level(self) -> &'static str;   // "exact" | "nameonly"
}

impl From<ResolutionConfidence> for FlowConfidence {
    // Exact -> Exact; NameOnly -> NameOnly(FlowDoubt::CallNameOnly)
}
```

- Changes `CpgEdge::DataFlow` to `CpgEdge::DataFlow(FlowConfidence)`. `CpgEdge::is_data_flow` stays the **label-insensitive** predicate: `matches!(self, CpgEdge::DataFlow(_))`.
- Step 4 (`src/cpg/build.rs:578`) temporarily emits the constant `DataFlow(FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete))`. Step 5b (`:1000`, `:1461`) emits `DataFlow(FlowConfidence::from(resolved.confidence))` **after** the existing `R6MultiOwnerCandidate` exclusion (`:923`).
- `edge_kind` (`src/navigation/queries.rs:2060`) becomes `CpgEdge::DataFlow(_) => "DataFlow"` — the wire string is unchanged, because `parse_ego_edges` (`:2071-2077`) parses it back and `src/mcp/tools.rs:352` enumerates it.

**Steps:**

- [ ] **Step 1: Re-derive the census from the tree, not from the spec.**

```bash
cd /Users/wesleyjinks/code/slicing-item2
grep -rn 'CpgEdge::DataFlow' src tests | tee /tmp/item2-dataflow-census.txt | wc -l
grep -rn 'CpgEdge::DataFlow' src tests | awk -F: '{print $1}' | sort | uniq -c | sort -rn
```

Expected at `bffb847`: **36 textual hits, 1 doc comment (`tests/infra/parallel_equality_test.rs:105`), 35 code sites in 14 files.** If Task 0b moved a line, enumerate the complete population before editing — never patch from the spec's numbers. Note: spec §2.1 says "9 are `==`/`!=` value comparisons" but enumerates **eleven**; the enumeration is right and the count is wrong. The eleven are `src/cpg/tests.rs:1242,1264,1385,2614,2675`, `src/cpg/multiline_call_arg_tests.rs:255,289,316`, `tests/common/mod.rs:134`, `tests/ast/cpg_test.rs:184`, `tests/integration/core_test.rs:873`.

- [ ] **Step 2: Write the `flow_confidence` unit tests first** in `src/cpg/flow_confidence.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolution::ResolutionConfidence;

    const ALL: [FlowConfidence; 6] = [
        FlowConfidence::Exact,
        FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 7 }),
        FlowConfidence::NameOnly(FlowDoubt::SameLine),
        FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
        FlowConfidence::NameOnly(FlowDoubt::AliasUnstable),
        FlowConfidence::NameOnly(FlowDoubt::CallNameOnly),
    ];

    #[test]
    fn worst_is_commutative_and_associative() {
        for a in ALL { for b in ALL {
            assert_eq!(a.worst(b), b.worst(a), "{a:?} vs {b:?}");
            for c in ALL {
                assert_eq!(a.worst(b).worst(c), a.worst(b.worst(c)), "{a:?} {b:?} {c:?}");
            }
        }}
    }

    #[test]
    fn nameonly_absorbs_exact() {
        for a in ALL.into_iter().skip(1) {
            assert_eq!(FlowConfidence::Exact.worst(a), a);
            assert!(!FlowConfidence::Exact.worst(a).is_exact());
        }
        assert_eq!(FlowConfidence::Exact.worst(FlowConfidence::Exact), FlowConfidence::Exact);
    }

    #[test]
    fn two_killed_doubts_keep_the_lower_kill_line() {
        let lo = FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 });
        let hi = FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 9 });
        assert_eq!(lo.worst(hi), lo);
        assert_eq!(hi.worst(lo), lo);
    }

    /// Pins the derived-`Ord` trap explicitly (§7.2): `Exact` sorts FIRST, so
    /// `min` returns the BEST label. `worst` must not be `min`.
    #[test]
    fn worst_is_not_std_cmp_min() {
        let a = FlowConfidence::Exact;
        let b = FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);
        assert_eq!(std::cmp::min(a, b), FlowConfidence::Exact, "derived Ord puts Exact first");
        assert_eq!(a.worst(b), b);
        assert_ne!(a.worst(b), std::cmp::min(a, b));
    }

    #[test]
    fn vocabulary_matches_the_finding_confidence_spelling() {
        assert_eq!(FlowConfidence::Exact.level(), "exact");
        for a in ALL.into_iter().skip(1) { assert_eq!(a.level(), "nameonly"); }
        assert!(FlowConfidence::Exact.is_exact());
    }

    #[test]
    fn resolution_confidence_conversion_maps_both_poles() {
        assert_eq!(FlowConfidence::from(ResolutionConfidence::Exact), FlowConfidence::Exact);
        assert_eq!(
            FlowConfidence::from(ResolutionConfidence::NameOnly),
            FlowConfidence::NameOnly(FlowDoubt::CallNameOnly)
        );
    }
}
```

- [ ] **Step 3: Write the parity test** `consumer_edge_sets_ignore_payload` in `tests/integration/dfg_label_parity_test.rs`. Build the `src/navigation` corpus (the same corpus `tests/infra/parallel_equality_test.rs:111` uses) twice through a test-only relabel seam — once with every Step-4 label forced to `Exact`, once to `NameOnly(CfgIncomplete)` — and assert **identical selected endpoint sets** for all ten production consumers: `data_flow_cycles` (`src/cpg/query.rs:238`, over `strongly_connected_components` `:218`), `dfg_forward_reachable` (`:650`), `dfg_backward_reachable` (`:728`), `dfg_chop` (`:792`), `taint_neighbors` (`src/cpg/trace.rs:843`), `has_incoming_dataflow` (`src/reasoning/shape.rs:165`), `gradient_slice::slice` (`:110`), `circular_slice::slice` (`:239`), `CpgEdge::is_data_flow` (`src/cpg/types.rs:313`), and the Step-5b interprocedural floor (the `(file, function, function_start_line)`-differs count from `tests/infra/parallel_equality_test.rs:108-142`).

- [ ] **Step 4: Run RED part 1 — feature absence.**

Run: `cargo test --lib cpg::flow_confidence` and `cargo test --test integration dfg_label_parity_test::consumer_edge_sets_ignore_payload`
Expected: compile errors — `flow_confidence` module absent, payload construction invalid, parity module unregistered. Record as **feature absent**; per the RED-shape rule this is *not* the RED.

Task 1 has **two** semantic contracts and therefore two RED records: the lattice (`flow_confidence`) and label-insensitive edge selection (the parity test). One stub cannot close both — the parity test needs the payload on the enum, which is the 35-site port — so they are recorded separately (sol r2 W1).

- [ ] **Step 4b: RED record (a) — the lattice, assertion-level against a closed stub.**

**Symbol closure.** `cargo test --lib cpg::flow_confidence` exercises:

| Symbol the test exercises | Provided by |
|---|---|
| `mod flow_confidence;` + `pub use` in `src/cpg.rs` (so `crate::cpg::flow_confidence::*` resolves) | **stub** |
| `FlowConfidence` (+ `Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `PartialOrd`, `Ord`, `Hash`) — `Copy` is needed for the `const ALL: [FlowConfidence; 6]` array | **stub** |
| `FlowDoubt` all five variants, incl. `Killed { kill_line: u32 }` | **stub** |
| `FlowConfidence::worst` / `is_exact` / `level` | **stub** |
| `impl From<ResolutionConfidence> for FlowConfidence` — called by `resolution_confidence_conversion_maps_both_poles` | **stub** (v3 omitted this; it was the compile error sol found) |
| `crate::resolution::ResolutionConfidence` | **exists** — `src/resolution.rs:26-29` |
| `std::cmp::min` over `FlowConfidence` (needs `Ord`) | **stub**'s derives |

Add the full type surface with a deliberately wrong lattice — the `std::cmp::min` trap the test exists to catch — plus the conversion and the registration:

```rust
// src/cpg.rs — registration the RED command needs
mod flow_confidence;
pub use flow_confidence::{FlowConfidence, FlowDoubt};

// src/cpg/flow_confidence.rs — STUB, replaced in Step 5
use crate::resolution::ResolutionConfidence;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FlowConfidence { Exact, NameOnly(FlowDoubt) }

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum FlowDoubt {
    Killed { kill_line: u32 },
    SameLine,
    CfgIncomplete,
    AliasUnstable,
    CallNameOnly,
}

impl FlowConfidence {
    /// STUB — deliberately the derived-`Ord` minimum, i.e. the BEST label.
    /// This is exactly the bug `worst_is_not_std_cmp_min` and
    /// `nameonly_absorbs_exact` must detect.
    pub fn worst(self, other: Self) -> Self { std::cmp::min(self, other) }
    pub fn is_exact(self) -> bool { matches!(self, FlowConfidence::Exact) }
    /// STUB — always "exact".
    pub fn level(self) -> &'static str { "exact" }
}

impl From<ResolutionConfidence> for FlowConfidence {
    /// STUB — collapses both poles to Exact.
    fn from(_c: ResolutionConfidence) -> Self { FlowConfidence::Exact }
}
```

Run `cargo test --lib cpg::flow_confidence`.
Expected: **assertion failures**, not compile errors —
- `nameonly_absorbs_exact`: left `Exact`, right `NameOnly(CfgIncomplete)`.
- `worst_is_not_std_cmp_min`: its `assert_ne!(a.worst(b), std::cmp::min(a, b))` fails, both sides `Exact`.
- `two_killed_doubts_keep_the_lower_kill_line`: got `Killed { kill_line: 4 }` where `min` picked by derived order, not by `kill_line`.
- `vocabulary_matches_the_finding_confidence_spelling`: got `"exact"`, expected `"nameonly"`.
- `resolution_confidence_conversion_maps_both_poles`: got `Exact`, expected `NameOnly(CallNameOnly)`.
- `worst_is_commutative_and_associative` **passes** (`min` is commutative and associative). That is the point: it is a structural test; the five value tests carry the semantics.

Record which failed and which did not.

- [ ] **Step 4c: RED record (b) — label-insensitive edge selection, assertion-level against a closed stub.**

**Symbol closure.** `cargo test --test integration dfg_label_parity_test::consumer_edge_sets_ignore_payload` exercises the whole ported surface, so the stub for this record **is the mechanical port**: `CpgEdge::DataFlow(FlowConfidence)` plus all 35 sites (Steps 6-7) plus the test-only relabel seam plus `tests/integration/main.rs` registration. That port is mechanical, not semantic — the *semantic* contract this test guards is "consumers select the same set regardless of payload", and the conservative stub makes exactly that wrong:

```rust
// src/cpg/types.rs — STUB for this RED record only. Replaced in Step 6.
impl CpgEdge {
    /// STUB — deliberately LABEL-SENSITIVE. The real predicate is
    /// `matches!(self, CpgEdge::DataFlow(_))`; this one selects only Exact
    /// edges, so the two relabelled builds disagree.
    pub fn is_data_flow(&self) -> bool {
        matches!(self, CpgEdge::DataFlow(FlowConfidence::Exact))
    }
}
```

Run the parity command.
Expected: **assertion failure** on the `is_data_flow` consumer — the all-`Exact` build selects the full DataFlow edge set, the all-`NameOnly(CfgIncomplete)` build selects the empty set, and the test reports the size difference. The other nine consumers **pass**, which localises the failure and proves the test can actually see a label-sensitive consumer rather than merely running. Record the failing consumer by name; this is the RED that shows §3 non-goal 1 is enforced by a test and not by a convention.

- [ ] **Step 5: Implement `src/cpg/flow_confidence.rs`** exactly as the Interfaces block. Register in `src/cpg.rs` beside the existing `mod build; mod cfg_queries; mod context;` list and re-export `FlowConfidence`/`FlowDoubt` so `crate::cpg::FlowConfidence` resolves. **Do not use `std::cmp::min` for the lattice.**

- [ ] **Step 6: Port the 14 production sites explicitly.** Producers (3): `src/cpg/build.rs:578` (Step 4, constant), `:1000` and `:1461` (Step 5b, `From<ResolutionConfidence>`). Predicates → `DataFlow(_)` (10): `src/cpg/build.rs:1325`; `src/cpg/query.rs:239, 679, 745, 811`; `src/cpg/trace.rs:852`; `src/algorithms/gradient_slice.rs:110`; `src/algorithms/circular_slice.rs:239`; `src/reasoning/shape.rs:165`; `src/cpg/types.rs:314`. Exhaustive arm (1): `src/navigation/queries.rs:2060`, wire string unchanged.

- [ ] **Step 7: Port the 21 test sites.** The eleven `==`/`!=` comparisons become payload-insensitive `matches!(…, CpgEdge::DataFlow(_))`. `tests/infra/parallel_equality_test.rs:121` destructures the payload without changing the interprocedural floor. `src/cpg/tests.rs:1138` (`CpgEdge::DataFlow.is_data_flow()`) and `:1142` become constructions with an explicit payload. Producers in `src/cpg/tests.rs:370, 449, 675, 676, 723, 763` take an explicit payload.

- [ ] **Step 8: Keep `FlowEdge` and `DataFlowGraph` unchanged in this task.** Note for reviewers: spec §4.1 justifies leaving `FlowEdge` alone partly by saying `delta_slice` "diffs `dfg.edges` by value". At `bffb847` that is **false** — `src/algorithms/delta_slice.rs:41-68` projects each edge to `(from.file, from.line, to.file, to.line)` and diffs the tuples. The decision still stands on the verified ground: ~10 synthetic `FlowEdge` construction sites have no label to supply (`src/algorithms/taint.rs:4930, 5055, 5359, 5401, 5473`; `src/cpg/cfg_queries.rs:264`; `src/cpg/query.rs:775`; `src/cpg/trace.rs:991`). Do not add a field to `FlowEdge`.

- [ ] **Step 9: Bump the cache once.**

```bash
rg -n 'CACHE_VERSION' src/cpg_cache.rs
```

Change `src/cpg_cache.rs:161` from `55` to `56`, move the pin at `:691` to `56`, and append **one** history entry:
`"v56: `CpgEdge::DataFlow` carries `FlowConfidence` from the reaching-definitions pass, and `DataFlowGraph` gains the `labels` map. Label-only — the edge set is unchanged."`
Then verify `rg -n 'NAV_CALL_EDGE_CACHE_VERSION' src/navigation/call_edge_cache.rs` still shows `24`. There is **no `#[serde(skip)]`** on the payload: bincode is non-self-describing, and `skip` would deserialize to `Default` and mint a fabricated label on every cache hit (§4.1, §5).

- [ ] **Step 10: Write `scripts/item2-byte-control.sh`.** Extend, do not replace: source the Phase 0 script's fixture enumeration (`:125`), `ALGOS` (`:153`), `FORMATS` (`:154`) and diagram case (`:185`); add `--format sarif` and `prism targets` invocations **with `--resolution nominal`** once Task 5 lands the flag (before Task 5 they run without it); assert a non-zero invocation count so an empty matrix cannot pass vacuously; compare stdout + stderr + exit status. Do not weaken `scripts/phase0-byte-control.sh`.

- [ ] **Step 11: Run GREEN.**

```bash
cargo test --lib cpg::flow_confidence
cargo test --test integration dfg_label_parity_test::consumer_edge_sets_ignore_payload
cargo test --test ast cpg_test::
cargo test --test integration core_test::
cargo test --test infra parallel_equality_test::
```

Expected: all PASS, each command reporting at least one test (a zero-test filter is inadmissible evidence).

- [ ] **Step 12: Byte controls.**

```bash
cargo build --release
scripts/phase0-byte-control.sh ~/code/tools/bin/prism-item2-0b target/release/prism
scripts/item2-byte-control.sh  ~/code/tools/bin/prism-item2-0b target/release/prism
```

Expected: zero differing invocations. A difference is a STOP.

- [ ] **Step 13: Full suite + gates** against the Task 0 baseline totals; `cargo fmt --all -- --check`; clippy delta.

- [ ] **Step 14: Commit.**

```bash
git commit -m "feat(item2): carry confidence on DataFlow edges" \
  -m "Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>" \
  -m "Claude-Session: https://claude.ai/code/session_015U8HwBTAFzFzqJbbq82JBT"
```

---

### Task 2: Reaching-definitions core pass, unwired

**Acceptance (spec §10 row 2):** §7.2 RD unit tests — diamond, loop, `kill_line`, caps, no-CFG — GREEN.

**Files:**

- Create: `src/cpg/reaching.rs` (strictly < 600 lines including tests; split before reaching 600).
- Modify: `src/cpg.rs` only to register the module.
- Read only: `src/cfg.rs`, `src/ast.rs`, `src/data_flow.rs`, `src/access_path.rs`.

**Interfaces:**

- Consumes: `cfg::build_cfg_edges_with_arms` (Task 0), `ParsedFile::statements_in_function` (`src/ast.rs:5778`), `ParsedFile::assignment_lvalue_spans_on_lines` (`:3609`), function parameter occurrences (`src/data_flow.rs:266-291`), `AccessPath`, and the alias semantics of `DataFlowGraph::build_alias_map` (`src/data_flow.rs:583`) / `resolve_path` (`:643`).
- **Bound before RED (not negotiable):**

```rust
/// Hard caps from the authorised measurement pass
/// (~/code/tools/logs/item2-census/REPORT.md §2.3, 92,338 functions).
/// RD_MAX_LINES bounds `stmt_lines.len()` — the CFG statement-line universe
/// returned by `ParsedFile::statements_in_function` — NOT the function's line
/// span (`end - start + 1`). Measured worst case: 590 defs, 331 statement
/// lines; 0 of 92,338 functions exceed either cap.
pub(crate) const RD_MAX_DEFS: usize = 2048;
pub(crate) const RD_MAX_LINES: usize = 4096;
```

  Also bound: `FlowConfidence` / `FlowDoubt` (Task 1) and the `labels` key shape `(VarLocation, VarLocation)` (Task 3).
- **Bound by sol r1 W3:** `Unavailable` must carry a **cap-specific reason** with **two distinguishable values** — one for the def cap, one for the line cap — so a test can prove *which* check fired. The exact shape is the implementer's proposal under B2 (an enum variant, a field, whatever fits); the requirement is that `over_cap_statement_lines_*` and `over_cap_defs_*` assert *different* observable reasons. Without this, a fixture of 4,097 assignment statements also exceeds `RD_MAX_DEFS`, and an implementation that never checks the line cap passes the line-cap test through the def cap.
- **Bound at Step 1 by the controller (owner B2):** the exact Rust signature of the RD entry point and its private types (`DefSite`, `DefId`, `Line`, `Unavailable`). The §4.2 pseudocode is binding for **semantics**, not signatures.

**Steps:**

- [ ] **Step 1: Propose the signatures; get controller approval; paste them here.** Write a task report containing the exact declarations, e.g. in this shape, and **do not proceed until the controller approves or amends**:

```rust
// PROPOSAL — controller fills in / amends, then this block becomes binding.
pub(crate) struct DefSite { pub id: DefId, pub path: AccessPath, pub line: Line, pub start_byte: usize, pub alias_derived: bool }
pub(crate) struct DefId(u32);
pub(crate) type Line = usize;
pub(crate) enum RdOutcome { Available(RdResult), Unavailable(RdUnavailable) }
pub(crate) enum RdUnavailable { OverCap, NoCfgEdges }
pub(crate) struct RdResult { /* IN sets per line, kill_at per def, same-line groups, capture set */ }
pub(crate) fn reaching_definitions(parsed: &ParsedFile, func_node: &Node<'_>, defs: &[DefSite], alias_map: &BTreeMap<String, String>) -> RdOutcome;
```

The report must say, for each private type, what §4.2 pseudocode line it realises. A guessed signature is a plan violation.

- [ ] **Step 2: Write failing GEN/KILL tests.**

```rust
#[test]
fn same_path_redefinition_kills_the_earlier_def() {
    // x = A (2); x = B (3); use(x) (4)  ->  A killed at 3, B live at 4
}
#[test]
fn alias_derived_defs_never_kill_in_v1() {
    // p = q; p.x = 1; q.x = 2; use(q.x)
    // v1 never uses a flow-insensitive alias relation as a kill proof (§4.2 line 14).
}
#[test]
fn same_line_defs_never_kill_one_another_by_byte_order() {
    // a = 1; a = 2;  on ONE physical line -> both marked collapsed; neither kills
    // the other; `start_byte`/`end_byte` are never compared (§4.3, owner B8 iii).
}
```

- [ ] **Step 3: Write failing fixpoint tests.**

```rust
#[test]
fn diamond_kill_on_one_branch_still_reaches() {
    // x=A; if c { x=B } else { }; use(x)
    // Standard RD union at the meet: A reaches along the else path -> Exact (§6 rule 4).
}
#[test]
fn loop_back_edge_makes_a_textually_earlier_use_reachable() {
    // for ...: use(x); x = f()
    // `collect_loop_back_edges` (src/cfg.rs:247) already supplies the back edge,
    // so the later def lands in IN[earlier use] -> Exact + loop_carried telemetry.
}
#[test]
fn kill_line_is_the_lowest_reachable_killing_line() {
    // x=A(2); if c { x=B(4) } else { x=C(6) }; use(x)(8)
    // kill_line == 4, not 6, and not the max.
}
#[test]
fn an_unreachable_kill_does_not_supply_the_payload() {
    // a killing redefinition on a branch that cannot be reached from the def
}
```

- [ ] **Step 4: Write failing availability tests — three DECOUPLED cap fixtures (sol r1 W3).**

Each cap fixture varies **one** quantity and holds the other below its cap, and each asserts the **cap-specific reason**, so neither check can pass through the other:

```rust
/// (a) LINE CAP, with the def count held at ZERO. 4,097 statement lines that
/// generate no Defs at all — bare calls, not assignments — so `defs.len() == 0`
/// and `RD_MAX_DEFS` cannot possibly fire. If the implementation omits the
/// `stmt_lines.len()` check this test fails, which is the whole point: a
/// fixture of 4,097 *assignments* would also blow the def cap and let a
/// line-cap-blind implementation pass.
#[test]
fn over_cap_statement_lines_returns_unavailable_for_the_lines_reason() {
    let mut src = String::from("def f():\n");
    for _ in 0..4097 { src.push_str("    g()\n"); }   // statements, zero Defs
    let (outcome, stats) = run_rd_on_single_function(&src, Language::Python);
    assert_eq!(defs_len(&src), 0, "fixture must generate zero Defs, else the def cap confounds this test");
    assert_eq!(stmt_lines_len(&src), 4097);
    assert!(
        matches!(outcome, RdOutcome::Unavailable(r) if r.is_line_cap()),
        "expected the LINE-cap reason, got {outcome:?}"
    );
    assert_eq!(stats.functions_over_cap, 1);
}

/// (b) DEF CAP, with the statement-line count held UNDER 4,096. 2,049 Defs
/// packed onto ~700 statement lines (three assignments per line), so
/// `RD_MAX_LINES` cannot fire and only the def check can explain the result.
#[test]
fn over_cap_defs_returns_unavailable_for_the_defs_reason() {
    let mut src = String::from("def f():\n");
    for i in 0..683 { src.push_str(&format!("    a{i} = 1; b{i} = 2; c{i} = 3\n")); }
    let (outcome, stats) = run_rd_on_single_function(&src, Language::Python);
    assert!(defs_len(&src) > 2048);
    assert!(stmt_lines_len(&src) <= 4096, "fixture must stay under the line cap");
    assert!(
        matches!(outcome, RdOutcome::Unavailable(r) if r.is_def_cap()),
        "expected the DEF-cap reason, got {outcome:?}"
    );
    assert_eq!(stats.functions_over_cap, 1);
}

/// (c) LONG SPAN with few of both. RD_MAX_LINES bounds `stmt_lines.len()`, NOT
/// the function's line span. ~5,000 source lines that are almost all comments
/// and blanks, leaving ~20 statement lines and ~10 Defs. (REPORT.md §2.3: under
/// the *span* reading the observed maximum is 3,296 and two of 92,338 functions
/// exceed 2048 — a different question, deliberately not the one the cap asks.)
#[test]
fn a_long_span_with_few_statements_and_few_defs_is_not_unavailable() {
    let outcome = run_rd_on_single_function(&long_span_source(5000, 20, 10), Language::Python).0;
    assert!(matches!(outcome, RdOutcome::Available(_)), "got {outcome:?}");
}

#[test]
fn cap_poles_remain_runnable() {
    // exactly 2048 Defs (under 4096 lines) -> Available
    // exactly 4096 statement lines (zero Defs) -> Available
}

#[test]
fn zero_cfg_edge_function_returns_unavailable_for_the_no_cfg_reason() {
    // build_function_cfg returns early on an empty statement list
    // (src/cfg.rs:40-42) -> a reason distinct from both cap reasons, and
    // functions_without_cfg == 1 while functions_over_cap == 0.
}
```

- [ ] **Step 5: Write failing capture-classification tests.** Minimal parsed sources for: Python `lambda` and nested `def`; Go `defer`/`go` function literal; JS/TS arrow and `function`; Rust closure. Assert each captured outer-binding read inside the delayed body classifies incomplete. Assert the paired negative: **`defer f(x)` evaluates `x` now**, so it is an ordinary argument read and remains eligible for its normal RD label (§4.2, §7.1 `dfg_reaching_defer_argument_now`). Each test's name states which construct made the timing unknown (owner B8 (i)).

- [ ] **Step 6: Run RED — both parts (RED-shape rule).**

**Part 1 — feature absence.** Run `cargo test --lib cpg::reaching::tests`. Expected: compile error, module absent. Record as **feature absent**; not the RED.

**Part 2 — assertion-level failure against a conservative stub.**

**Symbol closure (sol r2 W1).** `cargo test --lib cpg::reaching::tests` exercises:

| Symbol the tests exercise | Provided by |
|---|---|
| `RdOutcome`, `RdUnavailable`, `RdResult`, `DefSite`, `DefId`, `Line` | **Step 1's controller-approved declarations** — they land in the module before any test is written |
| `RdUnavailable::is_line_cap()` / `is_def_cap()` (two distinguishable reasons) | Step 1's approved shape; if the approved shape uses match patterns instead of predicates, the tests use those patterns — the requirement is only that the two reasons are distinguishable |
| `reaching_definitions(...)` | **stub** |
| `RD_MAX_DEFS`, `RD_MAX_LINES` | **bound in Interfaces** |
| in-module test helpers `run_rd_on_single_function`, `defs_len`, `stmt_lines_len`, `long_span_source` | written with the tests in Steps 2-5; `run_rd_on_single_function` parses the source, collects `DefSite`s, calls `reaching_definitions`, and tallies `RdFileStats` |
| `RdFileStats { functions_over_cap, functions_without_cfg }` | Task 3's Interfaces; declared here in Step 1 so the counters are assertable |
| `ParsedFile::parse`, `Language::Python`, `Node`, `AccessPath`, `BTreeMap` | **exists** |

Add the smallest stub that compiles and answers pessimistically everywhere:

```rust
/// STUB — every function is Unavailable with the no-CFG reason and no labels.
/// Conservative (nothing is ever Exact) and deliberately wrong. Replaced in Step 7.
pub(crate) fn reaching_definitions(
    _parsed: &ParsedFile, _func_node: &Node<'_>, _defs: &[DefSite],
    _alias_map: &BTreeMap<String, String>,
) -> RdOutcome {
    RdOutcome::Unavailable(RdUnavailable::NoCfgEdges)
}
```

Run the same command. Expected assertion failures, each naming values:
- `over_cap_statement_lines_returns_unavailable_for_the_lines_reason` — `Unavailable`, but the **wrong reason** (`NoCfgEdges`, not the line-cap reason). This is exactly the discrimination W3 asked for: a stub that returns `Unavailable` unconditionally must not be able to pass a cap test.
- `over_cap_defs_returns_unavailable_for_the_defs_reason` — same, wrong reason.
- `a_long_span_with_few_statements_and_few_defs_is_not_unavailable` and `cap_poles_remain_runnable` — got `Unavailable`, expected `Available`.
- Every GEN/KILL, diamond, loop, `kill_line`, same-line and capture test — no labels produced.
- `zero_cfg_edge_function_returns_unavailable_for_the_no_cfg_reason` **passes** against the stub. Record that: it is the one test the stub satisfies by accident, and it is therefore the weakest of the set on its own.

Preserve both observations in the task report.

- [ ] **Step 7: Implement §4.2 exactly.** Synthetic ENTRY at the function start line with `succ(ENTRY) = {first(stmt_lines)}` — parameter Defs are pinned to the signature line (`src/data_flow.rs:274`), which is **not** in `stmt_lines` because `collect_statements` walks the body (`src/ast.rs:5785`). CFG adjacency from `build_cfg_edges_with_arms` restricted to this function's line range. Fixed-width bit-vectors of width `defs.len()` (REPORT.md §2.2: the largest observed function needs 5.4 KiB per family; do **not** switch to a sparse `BTreeSet<DefId>` — REPORT.md §2.3 shows its trigger fires on zero functions and at p90 a dense vector is one `u64`). GEN/KILL per §4.2 lines 10-14. Worklist fixpoint in reverse postorder from ENTRY, `IN[L] = ⋃ OUT[P]` over predecessors **including back edges**.

- [ ] **Step 8: Implement the labeling precedence exactly** (§4.3): capture/deferred-body `CfgIncomplete` > ordinary `CfgIncomplete` > `SameLine` > `AliasUnstable` > `Killed`. A reason that says "RD could not run" must never be reported as "RD proved a kill". An edge crossing a flagged CFG join — try-header exception/finally (`src/cfg.rs:337-365`, `:525-570`), Go synthetic return→defer (`:379`), or `ArmProvenance::crosses_lexical_arm()` from Task 0 — is `NameOnly(CfgIncomplete)`.

- [ ] **Step 9: Implement same-line grouping** on `(AccessPath, line)` at line-granular identity; mark **every** member of a group with ≥2 Defs; never compare `start_byte`/`end_byte` to pick a winner.

- [ ] **Step 10: Run GREEN.**

Run: `cargo test --lib cpg::reaching::tests`
Expected: every GEN/KILL, diamond, loop, `kill_line`, cap, span-vs-statement-count, no-CFG, same-line, capture, and immediate-defer case passes.

- [ ] **Step 11: Gates.** `cargo fmt --all -- --check`; `cargo clippy --lib --all-features -- -D warnings`. No byte control is required — the pass is not wired and cannot change output.

- [ ] **Step 12: Commit.**

```bash
git add src/cpg/reaching.rs src/cpg.rs
git commit -m "feat(item2): add the reaching-definitions core" \
  -m "Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>" \
  -m "Claude-Session: https://claude.ai/code/session_015U8HwBTAFzFzqJbbq82JBT"
```

---

### Task 3: Wire RD labels into the DFG/CPG, commit the fixtures as staged RED, close the cache lifecycle

**Acceptance (spec §10 row 3, owner B6):** §7.1 `dfg_reaching_*` fixtures and expected labels **committed as staged RED** — the `nav dfg-stats --edges` oracle arrives in Task 6, so the matrix cannot be GREEN here and this task never claims it is. §7.2 label unit tests GREEN; §7.4 single-source parity GREEN; §5.1 byte control still zero diffs.

**Files:**

- Modify: `src/data_flow.rs` (`labels`, `rd_function_stats`, lifecycle at `:121`, `:139`, `:157`, `:169`, `:188`, `:209`), `src/cpg/reaching.rs`, `src/cpg/build.rs` (Step 4 derivation at `:549-579`), `src/cpg.rs`, `src/cpg_cache.rs` (persistence plumbing only — version stays 56), `tests/ast/dfg_test.rs`, `tests/integration/dfg_label_parity_test.rs`, `tests/integration/main.rs`.
- Modify: `eval/tier_a/matrix.py` (`PROBE_TYPES` `:27`, `KNOWN_TOP_SECTIONS` `:87`, `EXPECT_KEYS_BY_PROBE` `:90`, `Case` `:31`, `load_case` `:112`, `_run_matrix_inner` `:454`).
- Create: `eval/fixtures/<lang>/dfg_reaching_*/` per §7.1.

**Interfaces:**

- Consumes: the Task 2 RD result, `DataFlowGraph::build_from_refs` (`src/data_flow.rs:209`), `build_subset` (`:188`), `remove_files` (`:139`), `merge` (`:157`), `rebuild_adjacency` (`:169`), `CodePropertyGraph::assemble_graph` (`src/cpg/build.rs:431`), `from_parts` (`:163`).
- Produces exactly:

```rust
pub struct DataFlowGraph {
    // existing fields unchanged: edges, defs, uses, forward, backward
    /// One label per unique `(from, to)` FlowEdge endpoint pair. PRIMARY state
    /// with the same file-partitioned lifecycle as `edges` — NOT derived, so it
    /// is not touched by `rebuild_adjacency`.
    pub labels: BTreeMap<(VarLocation, VarLocation), FlowConfidence>,
    /// Per-file RD availability counters. Not recoverable from `labels` (an
    /// over-cap function and an ordinary CFG gap both yield CfgIncomplete).
    pub rd_function_stats: BTreeMap<String, RdFileStats>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RdFileStats {
    pub functions_over_cap: usize,
    pub functions_without_cfg: usize,
}
```

- **Insert rule:** when two construction arms produce one `(from, to)` key, insert `FlowConfidence::worst(existing, new)` — never first-wins. (§7.4 fixes only the parity test's collapse; see the PartialHit analysis, "one gap the spec leaves open".)
- Step 4 derives its payload from `dfg.labels`; a missing lookup falls back to `NameOnly(CfgIncomplete)` and increments `dfg_rd_functions_without_cfg`, **never** `Exact` (§6 rule 10). Step 5b remains the only graph DataFlow edge absent from `dfg.edges`/`dfg.labels` and keeps its resolved-callee payload.

**Steps:**

- [ ] **Step 1: Extend the matrix harness before writing fixtures.** In `eval/tier_a/matrix.py`: add `"dfg"` to `PROBE_TYPES` (`:27`); add the `[expect.edges]` section to `KNOWN_TOP_SECTIONS` (`:87`) and `EXPECT_KEYS_BY_PROBE` (`:90`); extend `Case` (`:31`) and `load_case` (`:112`) for the subset schema; add `_run_dfg_case` and an **explicit** `elif case.probe == "dfg":` branch in `_run_matrix_inner` before the `else:` at `:463`. The existing `else:` routes to module-deps and would let a `dfg` case pass vacuously — that is the specific failure §7.1 calls out. `_run_dfg_case` must fail when the expected-edge list is empty.

- [ ] **Step 2: Commit every §7.1 fixture with its expected labels.** One directory per row, smallest source that constructs the edge, one `expected.toml`, `probe = "dfg"`:

```toml
[case]
language = "python"; capability = "dfg_reaching_killed_def"; status = "pass"; probe = "dfg"
[[expect.edges]]
from = "a.py:2:x"; to = "a.py:4:x"; confidence = "nameonly"; doubt = "killed"; kill_line = 3
```

The complete row set, each asserting the label named in §7.1:

| Fixture | Languages | Asserted |
|---|---|---|
| `dfg_reaching_killed_def` | py, go, rs, js, ts | `A→use` = `nameonly/killed{kill_line = B's line}`; `B→use` = `exact` |
| `dfg_reaching_loop_carried` | py, go, rs, js, ts | later-def → earlier-use = `exact`; `dfg_label_loop_carried ≥ 1` |
| `dfg_reaching_shadowed_inner` | go, rs, js, ts | no outer-def→inner-use edge (pre-existing `is_shadowed_at`, `src/ast.rs:5134`); outer-def→outer-use = `exact` |
| `dfg_reaching_shadowed_inner_negative` | go, rs, js, ts | a **non-empty** NameOnly finding with `crossed_unlabeled = true` for the inner shadow boundary, **and** separately that the outer-def→inner-use edge is absent |
| `dfg_reaching_alias_conservative` | py, go, rs | retained alias edge = `nameonly/alias_unstable` even though `p` is assigned once |
| `dfg_reaching_alias_unstable` | py, go | `nameonly/alias_unstable` |
| `dfg_reaching_same_line` | py, js, ts | non-empty `SameLine` set; every collapsed endpoint `nameonly/sameline`; **no** `exact` edge in the group |
| `dfg_reaching_nonlocal_global` | py | the module/call-boundary DFG edge stays **absent**; no label minted; the finding projection through the unproved binding is `unlabeled/candidate` |
| `dfg_reaching_capture_timing` | py, go, js, ts, rs | every outer-def→capture-read edge `nameonly/cfg_incomplete`; explicit late-binding negative forbidding `exact` |
| `dfg_reaching_defer_argument_now` | go | `x→defer-argument` asserted `exact` |
| `dfg_reaching_go_short_var_if` | go | header `v`-def→in-block use `exact`; outer `v`-def→in-block use `nameonly` |
| `dfg_reaching_js_var_hoisting` | js | pre-def use edge `nameonly`; **not** deleted |
| `dfg_reaching_rust_let_shadow` | rs | first def→use `nameonly/killed`; second `exact` |
| `dfg_reaching_cfg_gap` | py, ts | continuation-line edge `nameonly/cfg_incomplete` |
| `dfg_reaching_interproc_exact` / `_nameonly` | py, go | Step-5b `exact` / `nameonly/call_nameonly` |
| `dfg_reaching_cfg_try_header_negative` | py + JS/TS/Java shape | `x=source; try: x=clean; raise; except: sink(x)` — the try-header→handler/finally join is **non-empty** and `nameonly/cfg_incomplete` |
| `dfg_reaching_cfg_go_defer_negative` | go | `x=source; defer f(x); x=clean; return` — the synthetic return→defer reach is **non-empty** and `nameonly/cfg_incomplete` |
| `dfg_reaching_cfg_branch_arm_negative` | py | `x=source; if c: x=clean; else: sink(x)` — any cross-arm sequential edge is **non-empty** and `nameonly/cfg_incomplete` (needs Task 0's `ArmProvenance`) |

Every Exact fixture has a paired NameOnly delivery case whose artifact is **non-empty** — asserting only that Exact is absent is not a pair (§7.1).

- [ ] **Step 3: Record staged RED (owner B6).**

```bash
cd /Users/wesleyjinks/code/slicing-item2 && cargo build --release
cd eval && uv run tier-a --matrix-only --allow-stale-sut 2>&1 | tee /tmp/item2-task3-matrix-red.log
```

Expected, in order: the harness first rejects unknown probe `dfg` until Step 1 lands, then the cases fail because `prism nav dfg-stats` does not exist until Task 6. **Record both as staged RED.** Task 3 makes no matrix acceptance claim; its semantic GREEN is Steps 8-11 below.

- [ ] **Step 4: Write the failing DFG store tests** in `tests/ast/dfg_test.rs`:

```rust
#[test]
fn labels_have_exactly_one_entry_per_unique_edge() {
    // key set of dfg.labels == unique (from,to) set of dfg.edges — asserted
    // after a cold build.
}
#[test]
fn labels_survive_remove_files_and_merge_with_the_same_membership() {
    // cold build -> remove_files({"b.py"}) -> assert key parity again
    // -> merge(build_subset(files, {"b.py"})) -> assert key parity again.
    // This is the guard for the lifecycle-carried store (plan's B5 analysis).
}
#[test]
fn a_synthetic_flow_edge_lookup_is_none_not_a_default() {
    // constructing a FlowEdge by hand and looking it up returns None; it must
    // never resolve to FlowConfidence::Exact via Default.
}
#[test]
fn rd_function_stats_are_file_partitioned() {
    // remove_files drops the entry for the removed file; merge reinstates only
    // the fresh file's entry; retained files' counters are untouched.
}
```

- [ ] **Step 5: Run RED.**

Run: `cargo test --test ast dfg_test::labels_` and `cargo test --test integration dfg_label_parity_test::`
Expected: compile errors — no `labels` field, no `rd_function_stats`; and the parity test still sees Task 1's single constant label, so it cannot observe semantic variation.

- [ ] **Step 6: Build `labels` beside `edges` inside each per-file DFG build.** In `build_from_refs` (`src/data_flow.rs:209`), after the per-file `file_refs` walk, run the approved RD pass per function and label **only edges that already exist**. Never call `edges.push` from the RD path. Preserve deterministic merge order (BTree everywhere). Insert with `worst()` on collision.

- [ ] **Step 7: Close the lifecycle.** `empty` (`:121`) starts `labels`/`rd_function_stats` empty. `remove_files` (`:139`) retains a label only when neither endpoint's file is in `exclude`, and drops `rd_function_stats` entries whose key is in `exclude`. `merge` (`:157`) extends both. `rebuild_adjacency` (`:169`) does **not** touch either — they are primary, not derived. No merge may manufacture a default label.

- [ ] **Step 8: Derive the Step 4 payload.** At `src/cpg/build.rs:578`, replace the Task 1 constant with a lookup in `dfg.labels` keyed by the same `(edge.from, edge.to)` pair the `var_index` keys are built from (`:559-575`). Keep endpoint lookup and edge insertion byte-identical. On a missing label use `NameOnly(CfgIncomplete)` and count it.

- [ ] **Step 9: Upgrade the parity test.** `tests/integration/dfg_label_parity_test.rs::consumer_edge_sets_ignore_payload` moves from Task 1's two-constant harness to **real RD labels vs. forced `NameOnly(CfgIncomplete)`**, comparing the same ten production consumer endpoint sets. Labels differ; sets must not.

- [ ] **Step 10: Add single-source parity (§7.4).**

```rust
#[test]
fn every_graph_dataflow_payload_equals_its_dfg_label() {
    // For each CpgEdge::DataFlow(c) whose endpoints map back to VarLocations:
    //   if dfg.labels has the (from,to) key -> assert c == that label
    //   else -> prove it is a Step-5b cross-function edge (endpoints differ in
    //           (file, function, function_start_line)) and equals the
    //           resolved-callee floor.
    // Parity is over UNIQUE (from,to) pairs; duplicate graph observations
    // collapse via .or_insert, never by choosing a byte occurrence (§7.4).
}
```

- [ ] **Step 11: Add the cache lifecycle test (§7.5) — the B5 acceptance.**

```rust
#[test]
fn cache_cold_full_hit_and_partial_hit_agree_on_every_label() {
    // 1. cold build over a 3-file repo; save_cache; record labels + every
    //    CpgEdge::DataFlow payload.
    // 2. load_cache -> CacheResult::Hit; assert identical.
    // 3. edit exactly ONE file; load_cache -> CacheResult::PartialHit;
    //    build_incremental...; assert identical to a COLD REBUILD of the
    //    edited tree — retained, removed, AND recomputed edges.
    // 4. assert no label is missing and none is a Default.
}
```

- [ ] **Step 12: Run GREEN.**

```bash
cargo test --lib cpg::reaching::tests
cargo test --test ast dfg_test::
cargo test --test integration dfg_label_parity_test::
cargo test --test ast cpg_cache_test::
```

Expected: all PASS, each command reporting at least one test.

- [ ] **Step 13: Byte controls.** `scripts/phase0-byte-control.sh` and `scripts/item2-byte-control.sh` against `~/code/tools/bin/prism-item2-0b`. Expected zero differences. A membership, stdout, stderr, status, or nominal-golden delta is a STOP — labels must not change what is emitted.

- [ ] **Step 14: Full suite + gates.**

- [ ] **Step 15: Commit.**

```bash
git commit -m "feat(item2): label existing DFG edges with reaching defs" \
  -m "Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>" \
  -m "Claude-Session: https://claude.ai/code/session_015U8HwBTAFzFzqJbbq82JBT"
```

---

### Task 4: `EvidencePath` transport and per-finding confidence

**Acceptance (spec §10 row 4):** §7.2 and §7.6 GREEN, including Exact and NameOnly SARIF assertions; targets assertions only after Phase 0 Task 4 has landed; taint output bytes unchanged.

> **Finding-producer manifest (spec v6 §7.6 / gate 11; sol r1 W1).** At `bffb847`, exactly **four** CPG algorithms construct `SliceFinding`s: **`provenance`, `taint`, `membrane`, `echo`** (Phase 0 inventory `~/code/tools/grounding/finding-inventory.md`, re-verified by grep). `leftflow`, `fullflow`, `relevant`, `conditioned`, `barrier`, `chop`, `delta`, `spiral`, `circular`, `vertical`, `3d` and `gradient` emit **no** findings — and `EvidencePath` is defined only as index-aligned with `findings`, so there is nothing to align to. **Minting a finding for any of them would change nominal JSON/review bytes** and violate §3 non-goal 1 / §5; if that is ever wanted it is a separate designated behaviour commit, not item 2 work. Therefore:
> - **End-to-end finding + evidence pairs (§7.6, gate 11): the four producers only.**
> - **Traversal-only algorithms are covered instead by labeled-walk unit tests** on their reachability twins — `dfg_forward_reachable_labeled`, `backward_reachable_labeled`, and chop's on-path label fold — **plus the §7.3 parity test**. Their *evidence recording* still lands in this task (spec §10 row 4 names chop/leftflow/fullflow); what does not land is a finding.
>
> Verify the manifest before writing tests, do not take it on faith. The manifest is the **intersection** of two sets, so check both:
> ```bash
> # (1) every algorithm file that constructs a SliceFinding
> grep -rn 'SliceFinding *{' src/algorithms/ | awk -F: '{print $1}' | sort -u
> # (2) the CPG-needing algorithms
> sed -n '211,231p' src/slice.rs
> ```
> (1) returns **ten** files at `bffb847`: `absence_slice.rs` (4), `callback_dispatcher_slice.rs` (2), `contract_slice.rs` (12), `echo_slice.rs` (1), `membrane_slice.rs` (1), `peer_consistency_slice.rs` (1), `primitive_slice.rs` (2), `provenance_slice.rs` (1), `symmetry_slice.rs` (1), `taint.rs` (3). (2) returns the sixteen `needs_cpg()` algorithms. **Their intersection is the four**: `EchoSlice`, `MembraneSlice`, `ProvenanceSlice`, `Taint`. The other six producers (absence, callback-dispatcher, contract, peer-consistency, primitive, symmetry) are AST-only and keep the Phase 0 rule — empty `EvidencePath`, `Exact` — and are covered by Task 4 Step 1's `ast_only_algorithm_with_empty_evidence_stays_exact`. If either set differs from the above, enumerate it and report before proceeding: this is an empirical claim about this tree, not a definition.

**Files:**

- Modify: `src/finding_confidence.rs` (`classify` at `:159`, `RESOLUTION_MODE` at `:70`), `src/slice.rs` (`SliceResult` at `:299`), `src/data_flow.rs` (`backward_reachable` `:695`), `src/cpg/query.rs` (`dfg_forward_reachable` `:650`, `dfg_chop` `:792`), `src/cpg/trace.rs` (`parents_by_root` `:111`), `src/cpg/cfg_queries.rs` (`taint_forward_cfg` `:197`), `src/api/run.rs` (`ReviewRun` `:289`, `run_review` `:297`, `ReviewOutcome` `:344`), `src/api/mod.rs`.
- Modify (evidence producers): `src/algorithms/{provenance_slice,taint,chop,left_flow,full_flow,echo_slice,membrane_slice}.rs`.
- Modify (emitters): `src/output/sarif.rs` (`classify` call at `:306`), `src/output/sarif_model.rs`, `src/targets/mod.rs` (`classify` call at `:83`), `src/targets/model.rs`.
- Modify tests: `src/finding_confidence.rs` in-module, `tests/integration/dfg_label_parity_test.rs`; create `tests/cli/min_confidence_test.rs` and register it in `tests/cli/main.rs`.

**Interfaces:**

- Consumes: `DataFlowGraph.labels` (Task 3), `backward_reachable` (`src/data_flow.rs:695`), `dfg_forward_reachable` (`src/cpg/query.rs:650`), `dfg_chop` (`:792`), `Trace::parents_by_root` (`src/cpg/trace.rs:111`), `Relation` (`:33`), `CallGraph::resolved_caller_edges`, `ResolvedCallEdge.confidence` (`src/resolution.rs:172`).
- Produces exactly (§4.4):

```rust
#[derive(Debug, Clone, Default)]
pub struct EvidencePath {
    pub hops: Vec<EvidenceHop>,          // the SELECTED witness hops
    pub crossed_unlabeled: bool,         // an unlabelled/name-only bridge
}

pub enum EvidenceHop {
    DataFlow { from: VarLocation, to: VarLocation, confidence: FlowConfidence },
    Call { edge: ResolvedCallEdge, confidence: ResolutionConfidence },
}

pub fn classify_with_evidence(
    algorithm: &str,
    quality: ParseQuality,
    evidence: &EvidencePath,
) -> (FindingConfidence, FindingTier);

impl EvidencePath {
    /// The evidence-free artifact `classify` stands for.
    pub fn unlabeled_for(algorithm: &str) -> Self;
    /// Walk `Trace::parents_by_root` from `sink` back to `root`, taking the
    /// worst label over hops whose `Relation` is `DataFlow`.
    pub fn from_trace(trace: &Trace, root: NodeIndex, sink: NodeIndex) -> Self;
}
```

- `classify(a, q)` is defined **only** as `classify_with_evidence(a, q, &EvidencePath::unlabeled_for(a))`, so the two cannot drift.
- `SliceResult` (`src/slice.rs:299`) becomes `#[non_exhaustive]` and gains `#[serde(skip)] pub evidence: Vec<Option<EvidencePath>>`, index-aligned with `findings`. **`Some(EvidencePath { hops: vec![], crossed_unlabeled: false })` is valid only for an AST-only finding; `None` means the artifact is missing and classification is `Unlabeled/Candidate` — never an empty Exact path.** `SliceFinding` (`:23`) gains **no field**. This skip is safe because `SliceResult` never round-trips through the CPG cache; the `CpgEdge` payload is persisted and is **not** skipped.
- `ReviewRun` (`src/api/run.rs:289`) carries the flattened aligned `Vec<Option<EvidencePath>>`; `ReviewOutcome` (`:344`) carries it onward; `SarifInputs` (`src/output/sarif.rs:63`) and the Phase 0 targets consumer (`src/targets/mod.rs:37`) receive the same aligned pair plus the runtime mode.
- Labeled reachability twins return the existing result as their **first** component; the old method delegates once and discards the second, so there is exactly one walk implementation:

```rust
pub fn backward_reachable_labeled(&self, from: &VarLocation)
    -> (BTreeSet<VarLocation>, Vec<EvidenceHop>);
pub fn dfg_forward_reachable_labeled(&self, from: &VarLocation)
    -> (BTreeSet<VarLocation>, BTreeMap<NodeIndex, FlowConfidence>);
```

**Steps:**

- [ ] **Step 1: Write the classification tests first** in `src/finding_confidence.rs`:

```rust
#[test] fn unknown_algorithm_is_unlabeled_candidate() {}
#[test] fn crossed_unlabeled_is_unlabeled_even_with_exact_hops() {}
#[test] fn exact_hops_plus_clean_parse_are_exact_asserted() {}
#[test] fn any_nameonly_flow_or_call_hop_is_nameonly_candidate() {}
#[test] fn a_non_clean_parse_prevents_asserted() {}
#[test] fn ast_only_algorithm_with_empty_evidence_stays_exact() {}
#[test] fn a_missing_artifact_none_fails_to_unlabeled_not_empty_exact() {}
/// §7.2: the two entry points cannot drift.
#[test] fn classify_equals_classify_with_evidence_for_every_production_algorithm() {}
#[test] fn finding_confidence_nameonly_serializes_as_nameonly() {}
```

`Relation` is matched **exhaustively**: `DataFlow` consumes its label; `AssignmentPropagation`, `RecoveredDefUse`, `CallDescent`, `ReturnInput`, and `ReturnFlow` all set `crossed_unlabeled` unless a separately proved hop is added with its own regression test (§6 rule 9).

- [ ] **Step 2: Write the finding-level witness tests — the FOUR producers only (sol r1 W1).** Each of these emits a real `SliceFinding`, so each gets a non-empty finding **and** a non-empty evidence artifact:
  - **provenance — three cases (owner B7, spec §7.6).** (a) **Unlabeled:** the origin comes from `all_defs_of` (`src/algorithms/provenance_slice.rs:565`) ⇒ `crossed_unlabeled = true` ⇒ `unlabeled/candidate`, *even when the hop list is empty*. (b) **NameOnly:** start from the verified changed-use→definition relation and walk at least one NameOnly DataFlow hop (a killed or same-line definition) ⇒ `nameonly/candidate`. (c) **Exact:** the same relation over Exact hops only ⇒ `exact/asserted`.
  - **taint — two cases.** `EvidencePath::from_trace` follows the selected `parents_by_root` chain (`src/cpg/trace.rs:111`); one Exact chain and one NameOnly chain; `AssignmentPropagation`/`RecoveredDefUse` set `crossed_unlabeled`.
  - **echo — two cases** and **membrane — two cases.** Record the selected `ResolvedCallEdge` values from `resolved_caller_edges` (`src/algorithms/echo_slice.rs:182`, `src/algorithms/membrane_slice.rs:54`) and read each `.confidence` (`src/resolution.rs:172`). **Never infer confidence from `ConfidenceFilter::All`** — it is an admission predicate only (`src/cpg/query.rs:16-21`). Test Exact and NameOnly callers separately. Neither algorithm traverses a DataFlow edge; their evidence is call edges only.

- [ ] **Step 2b: Write the labeled-walk unit tests for the traversal-only algorithms (sol r1 W1).** These mint **no** finding — asserting one would require creating findings that do not exist today and would move nominal bytes. They are proved at the walk layer instead:

**These tests use fixtures whose labels are known by construction, and assert the exact hop endpoints and the exact worst-confidence value (sol r2 W2).** v3's versions were vacuous: `hops.iter().all(...)` passes on an empty `hops`, and "the map is non-empty" admits fabricated `Exact` labels beside a real `NameOnly` hop.

Two fixtures, both minimal Python. Their labels follow from §4.3 alone, so the expectations are derivable, not observed:

```rust
/// MIXED: one def with two uses, one on each side of a redefinition.
///   1  def f():
///   2      x = 1        # def A
///   3      sink(x)      # use U1   A→U1 : Exact (nothing kills between 2 and 3)
///   4      x = 2        # def B, kills A at line 4
///   5      sink(x)      # use U2   A→U2 : NameOnly(Killed { kill_line: 4 })
const MIXED: &str = "def f():\n    x = 1\n    sink(x)\n    x = 2\n    sink(x)\n";

/// PURE: a single Exact hop and nothing else.
///   1  def g():
///   2      y = 1        # def
///   3      sink(y)      # use     def→use : Exact
const PURE: &str = "def g():\n    y = 1\n    sink(y)\n";
```

```rust
/// FORWARD twin. `dfg_forward_reachable` walks graph DataFlow edges out of a
/// Def (`src/cpg/query.rs:650`); the labeled twin adds the worst label on each
/// target's discovering path and must not change the set.
#[test]
fn dfg_forward_reachable_labeled_carries_the_known_label_per_target() {
    let cpg = build_cpg(MIXED);
    let a = def_at(&cpg, "f.py", 2, "x");             // def A
    let u1 = use_at(&cpg, "f.py", 3, "x");
    let u2 = use_at(&cpg, "f.py", 5, "x");

    let plain = cpg.dfg_forward_reachable(&a);
    let (set, labels) = cpg.dfg_forward_reachable_labeled(&a);

    assert_eq!(set, plain, "the labeled twin must not change reachability");
    assert!(set.contains(&u1) && set.contains(&u2), "fixture must reach both uses, got {set:?}");

    // Exact per-target values, not merely "non-empty".
    assert_eq!(labels.get(&node_of(&cpg, &u1)), Some(&FlowConfidence::Exact));
    assert_eq!(
        labels.get(&node_of(&cpg, &u2)),
        Some(&FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 }))
    );
    // And the fold over the whole walk is the known worst.
    let worst = labels.values().copied().fold(FlowConfidence::Exact, FlowConfidence::worst);
    assert_eq!(worst, FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 }));
}

#[test]
fn dfg_forward_reachable_labeled_is_exact_on_a_pure_chain() {
    let cpg = build_cpg(PURE);
    let d = def_at(&cpg, "g.py", 2, "y");
    let (set, labels) = cpg.dfg_forward_reachable_labeled(&d);
    assert_eq!(set, cpg.dfg_forward_reachable(&d));
    assert_eq!(labels.len(), 1, "one target, got {labels:?}");
    assert!(labels.values().all(|c| *c == FlowConfidence::Exact));
}

/// BACKWARD twin. `DataFlowGraph::backward_reachable` (`src/data_flow.rs:695`)
/// reverses def→use edges with no assignment propagation, so a Use's backward
/// frontier is exactly its incoming Defs. MIXED gives U2 two of them with
/// DIFFERENT labels — the discriminating shape.
#[test]
fn backward_reachable_labeled_returns_both_incoming_hops_with_their_known_labels() {
    let dfg = build_dfg(MIXED);
    let a = def_loc(&dfg, "f.py", 2, "x");
    let b = def_loc(&dfg, "f.py", 4, "x");
    let u2 = use_loc(&dfg, "f.py", 5, "x");

    let plain = dfg.backward_reachable(&u2);
    let (set, hops) = dfg.backward_reachable_labeled(&u2);

    assert_eq!(set, plain, "the labeled twin must not change reachability");
    assert_eq!(hops.len(), 2, "U2 has exactly two incoming defs, got {hops:?}");

    // Exact endpoints, in BTree order of `from` (A@2 before B@4).
    let endpoints: Vec<(usize, usize, FlowConfidence)> = hops
        .iter()
        .map(|h| match h {
            EvidenceHop::DataFlow { from, to, confidence } => (from.line, to.line, *confidence),
            other => panic!("expected a DataFlow hop, got {other:?}"),
        })
        .collect();
    assert_eq!(
        endpoints,
        vec![
            (2, 5, FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 })),
            (4, 5, FlowConfidence::Exact),
        ],
        "hop endpoints and labels must be exact"
    );
    assert!(set.contains(&a) && set.contains(&b));

    let worst = endpoints.iter().map(|(_, _, c)| *c).fold(FlowConfidence::Exact, FlowConfidence::worst);
    assert_eq!(worst, FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 }));
}

#[test]
fn backward_reachable_labeled_is_exact_on_a_pure_chain() {
    let dfg = build_dfg(PURE);
    let u = use_loc(&dfg, "g.py", 3, "y");
    let (set, hops) = dfg.backward_reachable_labeled(&u);
    assert_eq!(set, dfg.backward_reachable(&u));
    assert_eq!(hops.len(), 1, "one incoming def, got {hops:?}");
    match &hops[0] {
        EvidenceHop::DataFlow { from, to, confidence } => {
            assert_eq!((from.line, to.line), (2, 3));
            assert_eq!(*confidence, FlowConfidence::Exact);
        }
        other => panic!("expected a DataFlow hop, got {other:?}"),
    }
}

/// CHOP fold. `dfg_chop` (`src/cpg/query.rs:792`) returns on-path `(file, line)`
/// pairs; the fold is the worst label over DataFlow edges with BOTH endpoints
/// on the path. `chop.rs` constructs no SliceFinding, so this is the contract.
#[test]
fn chop_on_path_fold_takes_the_worst_label_over_interior_edges_only() {
    let cpg = build_cpg(MIXED);
    let on_path = cpg.dfg_chop("f.py", 2, "f.py", 5);
    assert!(
        on_path.contains(&("f.py".into(), 2)) && on_path.contains(&("f.py".into(), 5)),
        "fixture must produce a non-empty chop, got {on_path:?}"
    );

    let fold = chop_label_fold(&cpg, &on_path);
    assert_eq!(
        fold,
        Some(FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 4 })),
        "the A→U2 interior edge is NameOnly(Killed) and must dominate the Exact A→U1"
    );

    // An edge with only ONE endpoint on the path must not contribute:
    // restrict the path to {2, 3} and the fold becomes Exact, because the
    // NameOnly edge's far endpoint (line 5) is now off-path.
    let narrowed: BTreeSet<(String, usize)> =
        [("f.py".to_string(), 2), ("f.py".to_string(), 3)].into_iter().collect();
    assert_eq!(
        chop_label_fold(&cpg, &narrowed),
        Some(FlowConfidence::Exact),
        "explored-and-abandoned hops never degrade a result"
    );
}

/// The name-based fallback branch bypasses the DFG entirely
/// (`left_flow.rs:104,116`; `full_flow.rs:142,167`) and can never be Exact.
#[test]
fn the_name_based_fallback_branch_always_sets_crossed_unlabeled() {
    // Fixture whose variable has references the DFG has no edge for, so the
    // fallback fires: assert the produced EvidencePath has
    // crossed_unlabeled == true and that classify_with_evidence yields
    // (Unlabeled, Candidate) even though `hops` is empty.
}
```

If the fixtures do not produce the stated labels once Task 3 lands, that is a **Task 3 defect to investigate, not an expectation to relax** — the labels above follow from §4.3's rule table applied to five lines of Python. Traversal-only algorithms are otherwise covered by the §7.3 parity test, which proves their selected edge sets are label-insensitive.

Task 4 has **two** semantic contracts — evidence-path *classification* and evidence *transport* — reached by different RED commands and touching disjoint symbol sets. One stub cannot close both, so they are two RED records (sol r2 W1).

- [ ] **Step 3: RED record (i) — classification. Part 1, feature absence.**

Run: `cargo test --lib finding_confidence`
Expected: compile error — `EvidencePath`, `EvidenceHop`, `classify_with_evidence`, `unlabeled_for` are absent. Record as **feature absent**; not the RED.

- [ ] **Step 3b: RED record (i) — classification. Part 2, assertion-level against a closed stub.**

**Symbol closure.** `cargo test --lib finding_confidence` exercises:

| Symbol the tests exercise | Provided by |
|---|---|
| `EvidencePath { hops, crossed_unlabeled }` + `Debug`, `Clone`, `Default` | **stub** |
| `EvidenceHop::{DataFlow { from, to, confidence }, Call { edge, confidence }}` + `Debug` | **stub** |
| `classify_with_evidence(&str, ParseQuality, &EvidencePath)` | **stub** |
| `EvidencePath::unlabeled_for(&str)` | **stub** |
| `classify(&str, ParseQuality)` | **exists** — `src/finding_confidence.rs:159` |
| `FindingConfidence`, `FindingTier`, `ParseQuality` | **exists** — `src/finding_confidence.rs:34, 45, 59` (all `#[non_exhaustive]`, all `Copy`) |
| `FlowConfidence`, `FlowDoubt` | **exists** — Task 1 |
| `ResolutionConfidence`, `ResolvedCallEdge` | **exists** — `src/resolution.rs:26, 169` |
| `VarLocation` | **exists** — `src/data_flow.rs:15` |
| `SlicingAlgorithm::from_str` (for the every-algorithm equivalence test) | **exists** — `src/slice.rs` |

```rust
// src/finding_confidence.rs — STUB, replaced in Step 4.
use crate::cpg::FlowConfidence;
use crate::data_flow::VarLocation;
use crate::resolution::{ResolutionConfidence, ResolvedCallEdge};

#[derive(Debug, Clone, Default)]
pub struct EvidencePath {
    pub hops: Vec<EvidenceHop>,
    pub crossed_unlabeled: bool,
}

#[derive(Debug, Clone)]
pub enum EvidenceHop {
    DataFlow { from: VarLocation, to: VarLocation, confidence: FlowConfidence },
    Call { edge: ResolvedCallEdge, confidence: ResolutionConfidence },
}

impl EvidencePath {
    /// STUB — the evidence-free artifact, always crossed.
    pub fn unlabeled_for(_algorithm: &str) -> Self {
        Self { hops: Vec::new(), crossed_unlabeled: true }
    }
}

/// STUB — always the most conservative answer. Deliberately wrong for the
/// Exact rows.
pub fn classify_with_evidence(_a: &str, _q: ParseQuality, _e: &EvidencePath)
    -> (FindingConfidence, FindingTier) {
    (FindingConfidence::Unlabeled, FindingTier::Candidate)
}
```

Run `cargo test --lib finding_confidence`. Expected **assertion failures** naming values:
- `exact_hops_plus_clean_parse_are_exact_asserted` — got `(Unlabeled, Candidate)`, expected `(Exact, Asserted)`.
- `any_nameonly_flow_or_call_hop_is_nameonly_candidate` — got `Unlabeled`, expected `NameOnly`.
- `ast_only_algorithm_with_empty_evidence_stays_exact` — got `Unlabeled`, expected `Exact`. **This is the one that proves the stub is wrong rather than merely conservative**, and it is why "always Unlabeled" is not an acceptable implementation.
- `classify_equals_classify_with_evidence_for_every_production_algorithm` — fails for every AST-only algorithm, where `classify` says `Exact` and the stub says `Unlabeled`.
- `crossed_unlabeled_is_unlabeled_even_with_exact_hops` and `a_missing_artifact_none_fails_to_unlabeled_not_empty_exact` **pass** against the stub — record them as the two a pessimistic stub satisfies by accident. No whole contract passes: the four failures above are the contract.

- [ ] **Step 3c: RED record (ii) — transport. Both parts.**

**Part 1, feature absence.** Run `cargo test --test integration dfg_label_parity_test::labeled_walk_`, `cargo test --test integration dfg_label_parity_test::finding_evidence_`, and `cargo test --test cli min_confidence_test::evidence_delivery_`. Expected: compile errors — the labeled twins, `SliceResult.evidence` and the aligned vector through `ReviewRun` are absent. Record as **feature absent**; not the RED.

**Part 2, assertion-level against a closed stub.** Symbol closure for those three commands:

| Symbol the tests exercise | Provided by |
|---|---|
| `CodePropertyGraph::dfg_forward_reachable_labeled` | **stub** |
| `DataFlowGraph::backward_reachable_labeled` | **stub** |
| `chop_label_fold` (test-side helper over `dfg_chop`'s on-path set) | **stub** |
| `EvidencePath::from_trace(&Trace, NodeIndex, NodeIndex)` | **stub** |
| `SliceResult.evidence: Vec<Option<EvidencePath>>` + updated `SliceResult::new` | **stub** — note `SliceResult` (`src/slice.rs:299`) is **not** `#[non_exhaustive]` today, so adding the field also means updating `SliceResult::new` (`:314`) and every struct-literal construction; `#[serde(skip)]` needs the field type to be `Default`, which `Vec` is |
| `ReviewRun.evidence` | **stub** — `ReviewRun` (`src/api/run.rs:289`) is **already** `#[non_exhaustive]`, so adding a field is source-compatible for external constructors |
| `ReviewOutcome` passthrough | **stub** — also already `#[non_exhaustive]` (`src/api/run.rs:344`) |
| `dfg_forward_reachable`, `backward_reachable`, `dfg_chop`, `Trace::parents_by_root`, `Relation` | **exists** — `src/cpg/query.rs:650`, `src/data_flow.rs:695`, `src/cpg/query.rs:792`, `src/cpg/trace.rs:111`, `:33` |
| `EvidencePath`, `EvidenceHop` | **Step 3b's stub**, already in the tree |

```rust
// STUB — correct sets, NO labels. Replaced in Step 5.
impl CodePropertyGraph {
    pub fn dfg_forward_reachable_labeled(&self, from: &VarLocation)
        -> (BTreeSet<VarLocation>, BTreeMap<NodeIndex, FlowConfidence>) {
        (self.dfg_forward_reachable(from), BTreeMap::new())
    }
}
impl DataFlowGraph {
    pub fn backward_reachable_labeled(&self, from: &VarLocation)
        -> (BTreeSet<VarLocation>, Vec<EvidenceHop>) {
        (self.backward_reachable(from), Vec::new())
    }
}
impl EvidencePath {
    /// STUB — no hops, and conservatively crossed.
    pub fn from_trace(_t: &Trace, _root: NodeIndex, _sink: NodeIndex) -> Self {
        Self { hops: Vec::new(), crossed_unlabeled: true }
    }
}
/// STUB — every finding's artifact is missing.
fn evidence_for(findings: &[SliceFinding]) -> Vec<Option<EvidencePath>> {
    vec![None; findings.len()]
}
```

Run the three commands. Expected **assertion failures**, not compile errors:
- `dfg_forward_reachable_labeled_carries_the_known_label_per_target` — the set assertion **passes**; `labels.get(&node_of(&cpg, &u1))` is `None`, expected `Some(Exact)`. Record both halves: the set half passing is what proves the failure is about labels.
- `dfg_forward_reachable_labeled_is_exact_on_a_pure_chain` — `labels.len()` is `0`, expected `1`.
- `backward_reachable_labeled_returns_both_incoming_hops_with_their_known_labels` — `hops.len()` is `0`, expected `2`. **This is the assertion v3 could not make**: the old `hops.iter().all(...)` passed vacuously on an empty vector.
- `backward_reachable_labeled_is_exact_on_a_pure_chain` — `hops.len()` is `0`, expected `1`.
- `chop_on_path_fold_takes_the_worst_label_over_interior_edges_only` — got `None`, expected `Some(NameOnly(Killed { kill_line: 4 }))`.
- The CLI `evidence_delivery_` cases — every finding classifies `unlabeled/candidate` because its artifact is `None`, so the Exact-delivery cases fail on value.

- [ ] **Step 4: Implement `EvidencePath`, `EvidenceHop`, and `classify_with_evidence`** per §4.4. Confidence is the worst over the **present** axes, mapped `FlowConfidence::Exact | ResolutionConfidence::Exact → Exact`, anything else → `NameOnly`; tier is `Asserted` iff confidence is `Exact` **and** `quality == Clean`. Do not serialize the artifact and do not add a field to `SliceFinding`.

- [ ] **Step 5: Add the labeled twins with one walk each.** `backward_reachable_labeled` in `src/data_flow.rs`, with `backward_reachable` (`:695`) redefined as its first component. `dfg_forward_reachable_labeled` in `src/cpg/query.rs`, with `dfg_forward_reachable` (`:650`) delegating and dropping the map. **The returned sets are unchanged**, which is why taint's output bytes do not move.

- [ ] **Step 6: Thread the aligned evidence** through `SliceResult` → `ReviewRun` → `ReviewOutcome` → `SarifInputs` and the targets consumer. Validate `findings.len() == evidence.len()` before any projection; on mismatch supply an explicit `None` (⇒ `Unlabeled`) for each unmatched finding plus a test-visible warning — never `Exact`. Record only the witness that produced each **emitted** finding; explored-and-abandoned hops do not degrade it.

- [ ] **Step 7: Run per-path end-to-end pairs (§7.6) — the four producers (sol r1 W1).** `taint`, `echo` and `membrane` each get a complete-Exact case (`exact/asserted`) and a complete-NameOnly case (`nameonly/candidate`); `provenance` gets its three cases (Unlabeled / NameOnly / Exact). **Nine end-to-end cases total.** Each emits a **non-empty** finding **and** a non-empty evidence artifact (or an explicit `crossed_unlabeled` artifact) that is inspected to prove *which* DataFlow hop, `ResolvedCallEdge.confidence`, or conservative boundary caused the result. Counters do not satisfy this. **`chop`, `leftflow`, `fullflow` and the other traversal-only algorithms are NOT in this step** — they mint no finding, and Step 2b's labeled-walk tests plus §7.3 parity are their coverage.

- [ ] **Step 8: Repeat the nine pairs through SARIF** (`src/output/sarif.rs:306`), asserting the `confidence`/`tier` fields. Targets assertions are gated on "Phase 0 Task 4 landed" — at `bffb847` `src/targets/**` exists, so record explicitly whether that literal prerequisite is met before claiming targets acceptance.

- [ ] **Step 9: Run GREEN — all four commands from both RED records.**

```bash
cargo test --lib finding_confidence                                        # RED record (i)
cargo test --test integration dfg_label_parity_test::labeled_walk_         # RED record (ii)
cargo test --test integration dfg_label_parity_test::finding_evidence_     # RED record (ii)
cargo test --test cli min_confidence_test::evidence_delivery_              # RED record (ii)
```

All PASS. Verify each command reports at least one test — a zero-test filter is inadmissible evidence.

- [ ] **Step 10: Byte controls.** Existing JSON/review bytes must not expose `evidence` (it is `#[serde(skip)]`); taint remains excluded only where Phase 0 already documented non-byte-stability.

- [ ] **Step 11: Full suite + gates. Step 12: Commit.**

```bash
git commit -m "feat(item2): classify findings from selected evidence" \
  -m "Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>" \
  -m "Claude-Session: https://claude.ai/code/session_015U8HwBTAFzFzqJbbq82JBT"
```

---

### Task 5: `--min-confidence`, `--resolution`, and emitter delivery

**Acceptance (spec §10 row 5):** §7.5 GREEN; §5.2 nominal-mode SARIF reproduces the same-base output; targets acceptance gated on "Phase 0 Task 4 landed".

**Files:**

- Modify: `src/cli.rs` (`ReviewArgs` `:50`, `TargetsArgs` `:241`), `src/main.rs`, `src/api/run.rs`, `src/api/mod.rs` (`RESOLUTION_MODE` re-export at `:57`), `src/finding_confidence.rs` (`RESOLUTION_MODE` at `:70`).
- Modify: `src/output/sarif.rs` (`:36`, `:249`, `:306`, `:345`), `src/output/sarif_model.rs`, `src/targets/mod.rs` (`:11`, `:83`, `:187`), `src/targets/model.rs` (`:34`), `docs/contracts/targets.schema.json` (descriptions only, as §4.5 requires).
- Modify: `tests/cli/min_confidence_test.rs`, `tests/cli/sarif_test.rs`, `tests/cli/targets_test.rs`, `tests/cli/confidence_test.rs`.

**Interfaces:**

- Consumes: `classify_with_evidence` (Task 4), the aligned finding/evidence vectors, `FindingConfidence`, `FindingTier`, `SarifInputs` (`src/output/sarif.rs:63`), `ReviewOutcome` (`src/api/run.rs:344`), `targets::project` (`src/targets/mod.rs:37`).
- Produces exactly — **clap `ValueEnum`s (owner B3)**, deliberately departing from `src/cli.rs`'s existing `String + value_parser = [...]` convention (`:64-70`, `:279`, `:283`) because the user is choosing from a closed set and clap should print the values in `--help`:

```rust
#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum MinConfidence {
    /// Only findings whose evidence path is entirely Exact.
    Exact,
    /// Exact, NameOnly AND Unlabeled. An unlabeled finding is not *below*
    /// nameonly, it is ungraded — dropping it by default would delete findings
    /// from today's output.
    #[value(name = "nameonly")]
    NameOnly,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum ResolutionMode {
    /// Report every CPG-derived finding as unlabeled/candidate. The
    /// conservative default and the byte-control mode.
    Nominal,
    /// Report the retained evidence-path labels.
    Scoped,
    // `precise` and `auto` are roadmap item 3 (SCIP/Tier 3); clap rejects them
    // and the long help says so. `auto` is deferred with a written reason: with
    // two modes and no external index it would be a synonym for `scoped`,
    // freezing a name whose meaning changes when item 3 lands.
}

// on ReviewArgs and TargetsArgs:
#[arg(long, value_enum, default_value_t = MinConfidence::NameOnly)]
pub min_confidence: MinConfidence,
#[arg(long, value_enum, default_value_t = ResolutionMode::Nominal)]
pub resolution: ResolutionMode,
```

- **`--resolution` default is `nominal`** for every finding-bearing surface (§4.5, unchanged v4→v6). *(Plan v1 said "scoped for slice/targets/sarif"; that contradicted the spec and was corrected in v2.)*
- `ResolutionMode` is a **runtime value** parsed by the CLI, stored in the API options, threaded through `run_review`/`ReviewOutcome`, and passed to `SarifInputs` and the targets consumer. **The `RESOLUTION_MODE: &str = "nominal"` constant at `src/finding_confidence.rs:70` is deleted**, along with its five reads (`src/output/sarif.rs:36,249,345`; `src/targets/mod.rs:11,187`) and its `src/api/mod.rs:57` re-export. §4.5: "There is no serializer read of a fixed `RESOLUTION_MODE` constant."
- Produces an **emit-time predicate only**: `NameOnly` admits Exact + NameOnly + Unlabeled; `Exact` admits only Exact. It never changes CPG construction, caches, algorithms, or evidence collection. Labels are always computed and always cached — one CPG serves both modes (roadmap §3.6); computing RD conditionally would create two cache states for one file set, which the per-diff cache (`src/cpg_cache.rs:10-13`) cannot key.

**Steps:**

- [ ] **Step 1: Write the byte-preservation RED cases** in `tests/cli/min_confidence_test.rs`, on a fixture with a known `nameonly` echo finding:

```rust
#[test] fn default_json_and_review_are_byte_identical_to_base() {}
#[test] fn min_confidence_exact_drops_the_nameonly_echo_finding() {}
#[test] fn default_nameonly_threshold_retains_an_unlabeled_finding() {}
```

- [ ] **Step 2: Write the SARIF RED cases.**

```rust
#[test] fn min_confidence_exact_sarif_has_only_exact_result_confidences() {}
#[test] fn resolution_nominal_sarif_reproduces_the_same_base_document() {}
#[test] fn resolution_scoped_sarif_reports_evidence_labels_and_mode_scoped() {}
```

`scoped` ⇒ `properties.resolution_mode = "scoped"` and CPG-derived findings carry `exact` or `nameonly`. `nominal` ⇒ every CPG algorithm is **forcibly** projected `unlabeled`/`candidate` regardless of its evidence; AST-only algorithms retain the Phase 0 rule.

- [ ] **Step 3: Write the targets RED cases** with the same Exact/NameOnly pairs: confidence/tier agree with SARIF; exact threshold retains only Exact; nominal reproduces the same-base confidence/tier/`resolution_mode` fields (`src/targets/model.rs:34,68-69`).

- [ ] **Step 4: Write the clap RED cases.**

```rust
#[test] fn a_bogus_confidence_value_exits_two_and_lists_the_possible_values() {}
#[test] fn resolution_precise_and_auto_are_rejected_and_name_roadmap_item_3() {}
#[test] fn min_confidence_is_rejected_for_text_paper_mermaid_and_callers() {
    // The error names the selected format AND says it has no stable finding
    // projection (owner B8 (ii); §4.5). Rejection happens during CLI
    // validation, BEFORE any work — never accept-then-silently-ignore.
}
```

- [ ] **Step 5: Run RED.** `cargo test --test cli min_confidence_test::`. Expected: unknown-flag errors; preserve stderr and exit codes.

- [ ] **Step 6: Implement the flags and delete the constant.** Resolve the mode once at the output boundary; never condition RD or cache construction on it.

- [ ] **Step 7: Replace every direct `classify` call** at `src/output/sarif.rs:306` and `src/targets/mod.rs:83` with the aligned `classify_with_evidence` result.

- [ ] **Step 8: Apply the filter before every projection** — single and multi JSON/review paths, SARIF, and targets — zipping findings with their `Vec<Option<EvidencePath>>` so selection, ordering, and alignment are identical for both vectors. No consumer filters findings independently.

- [ ] **Step 9: Run GREEN.** `cargo test --test cli min_confidence_test::`, the complete `tests/cli/sarif_test.rs`, the complete `tests/cli/targets_test.rs`, `tests/cli/confidence_test.rs`, and `cargo test --test integration api_test::`.

- [ ] **Step 10: Byte control** including default JSON/review plus nominal SARIF/targets. Expected zero differences: the default `nominal` + `nameonly` pair admits the same finding set as base.

- [ ] **Step 11: Full suite + gates. Step 12: Commit.**

```bash
git commit -m "feat(item2): filter emitted findings by confidence" \
  -m "Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>" \
  -m "Claude-Session: https://claude.ai/code/session_015U8HwBTAFzFzqJbbq82JBT"
```

---

### Task 6: `DfgLabelStats`, `nav dfg-stats`, docs/schema truth, and matrix GREEN

**Acceptance (spec §10 row 6, owner B6):** §7.1 matrix **GREEN** — this task owns Task 3's staged RED; §8 gate 9 non-inert fixture check per language; `tests/cli/call_stats_test.rs` still GREEN.

**Files:**

- Modify: `src/cpg/build.rs` (beside `ReturnFlowStats` `:20-33`, field `:136`), `src/cpg.rs`, `src/cpg_cache.rs` (persist beside `:213`, **no second version bump**), `src/navigation/queries.rs` (`call_stats` `:221`), `src/cli.rs` (`NavQuery` `:322`, beside `CallStats` `:397-403`), `src/main.rs` (`run_nav` `:43`, beside the `CallStats` arm `:164-178`).
- Modify: `eval/tier_a/matrix.py`, `tests/cli/call_stats_test.rs`, `tests/cli/min_confidence_test.rs`, `CLAUDE.md`, `README.md`, `docs/contracts/targets.schema.json`.

**Interfaces:**

- Consumes: the Task 3 labels and `rd_function_stats`, the Step-5b labels, `NavigationSession.index.cpg()`, `navigation::queries::call_stats` (`src/navigation/queries.rs:221`), the `NavQuery::CallStats` JSON assembly (`src/main.rs:164-178`), the Task 3 `dfg` matrix probe.
- Produces exactly (§4.7):

```rust
pub struct DfgLabelStats {
    pub dfg_label_exact: usize,
    pub dfg_label_loop_carried: usize,               // subset of exact
    pub dfg_label_nameonly_killed: usize,
    pub dfg_label_nameonly_sameline: usize,
    pub dfg_label_nameonly_cfg_incomplete: usize,
    pub dfg_label_nameonly_alias_unstable: usize,
    pub dfg_label_nameonly_call: usize,              // Step-5b, callee NameOnly
    pub dfg_rd_functions_over_cap: usize,
    pub dfg_rd_functions_without_cfg: usize,
}
```

- The seven per-label counters are **counted at Step 4 / Step 5b from the merged label state on every assemble** — not stored, so they cannot disagree with what shipped. The two function-level counters are summed from `DataFlowGraph.rd_function_stats` (Task 3), which survives `PartialHit` by file partition.
- `prism nav call-stats` gains an **additive** `"dfg_labels"` key beside `"return_flow"` (`src/main.rs:173-174`) — safe because `tests/cli/call_stats_test.rs` asserts structurally, not byte-wise.
- `NavQuery::DfgStats { repo: PathBuf, edges: bool }` prints the same object; `--edges` emits one JSON object per labeled DataFlow edge — `{from:{file,line,path,access}, to:{…}, confidence, doubt, kill_line}` — sorted by `(from, to)`, as JSONL, the exact shape of `call-stats --dump-sites`. **`--edges` is the by-construction oracle §7.1 needs; without it the fixtures have nothing to read.**
- `edge_kind(CpgEdge::DataFlow(_))` stays exactly `"DataFlow"`; the MCP edge grammar (`src/mcp/tools.rs:352`) and the nav sidecar format are unchanged.

**Steps:**

- [ ] **Step 1: Write the RED telemetry tests.** An empty graph emits all-zero counters; each fixture shape (killed / exact / loop / cfg-gap / alias / same-line / Step-5b) increments the matching field; `dfg_label_loop_carried ≤ dfg_label_exact`; the seven label counters sum to the labeled edge count (the two function counters are excluded from that sum).

- [ ] **Step 2: Write the `call_stats_test` addition:** `dfg_labels` is an object, and **every** pre-existing key is present and unchanged against a saved same-base control.

- [ ] **Step 3: Write the `min_confidence_test` CLI cases:** `nav dfg-stats` pretty JSON equals `call-stats.dfg_labels`; `--edges` is deterministic JSONL sorted by `(from, to)`, uses the exact doubt spelling, and includes `kill_line` **only** for `Killed`.

- [ ] **Step 4: Run RED.** `cargo test --test cli call_stats_test::dfg_labels_` and `cargo test --test cli min_confidence_test::dfg_stats_`. Expected: unknown `dfg-stats` subcommand; missing additive key.

- [ ] **Step 5: Implement `DfgLabelStats`** beside `ReturnFlowStats` (`src/cpg/build.rs:20-33`), as a field on `CodePropertyGraph` beside `:136`, persisted beside `src/cpg_cache.rs:213` **within v56** — no second bump.

- [ ] **Step 6: Add `NavQuery::DfgStats`** at `src/cli.rs` beside `CallStats` (`:397-403`) and its arm in `src/main.rs::run_nav` beside `:164`. For `--edges`, map graph variable endpoints back to file/line/path/access, include Step-5b edges, sort, then serialize one object per line.

- [ ] **Step 7: Add `dfg_labels` beside `return_flow`** in the `call-stats` assembly (`src/main.rs:173-174`). Change no existing leaf.

- [ ] **Step 8: Complete `_run_dfg_case` and turn every Task 3 fixture GREEN.** It must require at least one asserted edge or forbid clause, so an empty output cannot pass vacuously.

- [ ] **Step 9: Run GREEN.**

```bash
cargo test --test cli call_stats_test::
cargo test --test cli min_confidence_test::dfg_stats_
cargo build --release && cd eval && uv run tier-a --matrix-only --allow-stale-sut
```

Expected: the Task 0 base pass count **plus every new `dfg_reaching_*` case**, all `ok`.

- [ ] **Step 10: Run the non-inert gate (spec v6 §8.9, lesson 16).** Two halves:

**(a) Corpus half.** On at least one anchor corpus per language family — prism (Rust), caddy (Go), mypy (Python) — assert `dfg_label_exact > 0` **and** `dfg_label_nameonly_killed > 0`. A zero pole is a **STOP**, not a footnote: a shipped counter that reads zero everywhere means the mechanism is inert.

**(b) Fixture-pack half, scoped by spec v6.** The **five §7.1 fixture languages — py, go, rs, js, ts — must each have a non-empty Exact case and a non-empty NameOnly case in their own fixture pack**, not merely a corpus anchor; where the killed shape parses, `dfg_label_nameonly_killed > 0`. Every other language is **recorded** in this task's report with its census DataFlow-edge count, per the v6 ruling, and no pole is fabricated:

| language | disposition (spec v6 gate 9) | source |
|---|---|---|
| py, go, rs, js, ts | both poles required in the fixture pack | §7.1 |
| terraform | **gate 9 N/A — no DataFlow** (0 edges, 7/7 functions with 0 CFG nodes) | REPORT.md §G, §H |
| c, cpp, bash, tsx | carry edges; covered by the §7.3 parity test over their existing fixture packs; Exact/NameOnly poles filed as a follow-up | REPORT.md §H |
| java, lua | disposition from the **Task 0 Step 3b** census measurement — N/A if 0 DataFlow edges, otherwise the c/cpp/bash/tsx treatment | Task 0 Step 3b |

Carry the Task 0 Java/Lua table into this report verbatim so the gate record is self-contained.

- [ ] **Step 11: Truth pass.** `README.md` and `CLAUDE.md`: both modes, the `exact|nameonly|unlabeled` vocabulary, the filter surface and its default (including risk 5 — the default `nameonly` admits `unlabeled`, so the default gate is weaker than the flag name suggests), `nav dfg-stats`, the doubt vocabulary, cache v56, and `asserted` as an evidence-path claim. **Do not claim `precise`/`auto` support.**

- [ ] **Step 12: Verify the single transition.**

```bash
rg -n 'CACHE_VERSION|NAV_CALL_EDGE_CACHE_VERSION' src/cpg_cache.rs src/navigation/call_edge_cache.rs
git log -p -- src/cpg_cache.rs | rg 'CACHE_VERSION'
```

Expected: `56` and `24`; exactly one `55 → 56` edit on the branch and no intermediate value.

- [ ] **Step 13: Byte controls + the cold/full-hit/PartialHit label-parity test. Step 14: full suite + gates. Step 15: Commit.**

```bash
git commit -m "feat(item2): expose DFG confidence telemetry" \
  -m "Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>" \
  -m "Claude-Session: https://claude.ai/code/session_015U8HwBTAFzFzqJbbq82JBT"
```

---

### Task 7: Controller closeout — gates, corpus controls, roadmap row, handoff, review, PR

**Acceptance (spec §10 row 7):** all of §8.

**Files:**

- Create: `docs/superpowers/handoffs/2026-09-04-prism-item2-dataflow-confidence-handoff.md` from `~/.claude/handoff-template.md`.
- Modify: `docs/analysis/prism-post-plan-roadmap.md` — the §1 follow-up queue table (`| # | Item | Why / measured signal | Effort |`, rows 1-19 at `bffb847`): update row 19's "Next" pointer and **add row 20** (below).
- Read/verify: every Task 0–6 path, control artifacts, complete test/clippy logs, `eval/corpora.toml` anchor pins, the PR diff.

**Interfaces:**

- Consumes: Tasks 0–6 commits and evidence. Produces no analyzer behaviour.
- Produces: the complete gate record, the custody tuple, review verdicts with WRONG before SMELL, a durable handoff, and one PR against `shoedog/prism` main. **The controller, not an implementer, pushes and opens the PR.**

**Steps:**

- [ ] **Step 1: Freeze the candidate tuple.** Worktree, branch, `HEAD`, merge-base `bffb847`, dirty state, candidate binary sha256, `~/code/tools/bin/prism-base-bffb847` sha256, `~/code/tools/bin/prism-item2-0b` sha256, cache 56, nav sidecar 24.

- [ ] **Step 2: `cargo fmt --all -- --check`.** Expected exit 0, no diff.

- [ ] **Step 3: Clippy, with the same-environment control.**

```bash
cargo clippy --all-targets --all-features -- -D warnings 2>&1 | tee /tmp/item2-clippy-candidate.log
git stash && git checkout bffb847 && cargo clippy --all-targets --all-features -- -D warnings 2>&1 | tee /tmp/item2-clippy-base.log && git checkout - && git stash pop
```

Diff the normalized warning sets. New warnings block; pre-existing ones are recorded.

- [ ] **Step 4: Re-run every focused module from Tasks 0–6** and record passed/failed/ignored. **A zero-test filter is inadmissible** — verify each command reports at least one test.

- [ ] **Step 5: Full suite.**

```bash
cargo test --all-targets --all-features --no-fail-fast 2>&1 | tee /tmp/item2-suite-final.log
awk '/^test result:/{p+=$4; f+=$6; i+=$8; n++} END{printf "result_lines=%d passed=%d failed=%d ignored=%d\n",n,p,f,i}' /tmp/item2-suite-final.log
```

Expected: the **Task 0 `bffb847` baseline totals** (from `~/code/tools/logs/baseline-bffb847.log`, expected at or near `3810 / 0 / 1` over 29 result lines plus 2 doc-tests) plus the enumerated new test count, 0 failed. **Do not compare against `3543/0/1` — that is the pre-Phase-0 c220525 figure.** A failure outside this item's scope is reported, not re-baselined and not silently fixed.

- [ ] **Step 6: Byte controls.**

```bash
scripts/phase0-byte-control.sh ~/code/tools/bin/prism-item2-0b target/release/prism
scripts/item2-byte-control.sh  ~/code/tools/bin/prism-item2-0b target/release/prism
```

Expected: zero differing invocations across stdout, stderr, and exit status. Also record the Task 0b diff review (the one commit permitted to move bytes) against `prism-base-bffb847`.

- [ ] **Step 7: Cache gate 10.** `cargo test --test integration dfg_label_parity_test::cache_`. Expected: cold, full `Hit`, and one-file `PartialHit` all equal the cold edited-tree control for label membership and every DataFlow payload.

- [ ] **Step 8: Tier-A.**

```bash
cargo build --release && cd eval && uv run tier-a --matrix-only --allow-stale-sut
cd eval && uv run tier-a --quick --allow-stale-sut
```

Expected: base passes plus all `dfg_reaching_*` cases; no regression. Compare `--quick` against the same-base run. Do not rebaseline `docs/eval/tier-a/*.json`.

- [ ] **Step 9: Same-base call-stats control on all eleven anchors** — prism, ruff, ripgrep, caddy, cobra, prometheus, etcd, zap, black, httpx, mypy — with `prism nav --no-cache call-stats`. Diff leaf-by-leaf after excluding only the additive `dfg_labels`. **Every pre-existing key must be identical.** Any loss of Exact call edges or any changed pre-existing drop counter is a **STOP** under lesson 17 — do not explain it away as a DataFlow-only change.

- [ ] **Step 10: Non-inert checks** on prism, caddy, mypy: both `dfg_label_exact` and `dfg_label_nameonly_killed` nonzero in each recorded object.

- [ ] **Step 11: Verify the cache history and dependency set.** Exactly one `55 → 56` edit, one v56 history entry, sidecar 24, no `Cargo.toml`/`Cargo.lock` dependency change.

- [ ] **Step 12: Verify gate 11 against the four-producer manifest, then inspect artifacts, not exit statuses (sol r1 W1).**

Gate 11 is satisfied by, and only by: **nine** non-empty end-to-end finding + evidence delivery cases — `provenance` × 3 (Unlabeled / NameOnly / Exact), `taint` × 2, `membrane` × 2, `echo` × 2 — with aligned evidence and pre-projection filtering, each delivered through SARIF. **Do not look for, and do not accept, finding-level cases for `leftflow`, `fullflow`, `relevant`, `conditioned`, `barrier`, `chop`, `delta`, `spiral`, `circular`, `vertical`, `3d` or `gradient`: they construct no `SliceFinding` at `bffb847`, and one appearing would mean nominal bytes moved.** Their gate-11 obligation is discharged by Task 4 Step 2b's labeled-walk unit tests and the §7.3 parity test; record that disposition explicitly rather than leaving the twelve unaccounted for. Re-run the manifest grep from Task 4 and confirm the producer set is still those four.

Then inspect: SARIF and targets Exact/NameOnly pairs, nominal outputs, DFG JSONL, the complete test log, clippy logs, byte-control reports, matrix report, quick report, call-stats diffs, the Task 0b before/after CFG-admissibility table, and the Task 0 Java/Lua census table.

- [ ] **Step 13: Add the roadmap row.** In `docs/analysis/prism-post-plan-roadmap.md` §1, append row **20** to the follow-up queue table (owner-ruled follow-up from the runtime fault-injection harness; see `~/code/tools/03-tooling-plan-roadmap.md:131` §3 Phase 1, and `~/code/tools/DECISIONS.md` §C2/§E3):

```markdown
| 20 | **`prism targets` emits a `dependency_hint` for every `external_call` it can** (NEW 2026-09-04, owner-ruled at the item 2 plan re-anchor) — the runtime fault-injection harness treats a target with no `dependency_hint` as probe-only and counts it in coverage as `no-dependency-hint`, never as "inject the whole catalog" (which would be a false finding by construction). Prism side: widen `src/targets/mapping.rs` so every `external_call` target that can carry a hint (kind + callee) does. Harness side: runtime-harness spec §5.2 + CR-2. | `~/code/tools/03-tooling-plan-roadmap.md` §3 Phase 1; `~/code/tools/DECISIONS.md` C2/E3; `reviews/faultpath-scaffold-report.md` Q1 | S |
```

Also update row 19's trailing "**Next: roadmap item 2**" pointer to reflect item 2's actual state.

- [ ] **Step 14: Write the handoff** from `~/.claude/handoff-template.md` at this stable point: the custody tuple, commits, exact commands and outputs, totals, controls, known exclusions, the recorded rulings (B2–B8, E4, Task 0b), the Task 0b behaviour-commit evidence packet, and every remaining concern including the unresolved spec defects listed below. **Commit it before review so it is durable.**

- [ ] **Step 15: Dispatch whole-branch review, cap 2 rounds declared up front:** one spec-compliance seat and one adversarial implementation seat. Every finding tagged WRONG or SMELL, WRONG first, each with a concrete input/state and the incorrect result. The sol seat additionally rules on the PartialHit choice above and on the `worst()`-on-insert prescription.

- [ ] **Step 16: Fix waves.** For each valid closed finding, apply **one bounded fix wave to the existing artifact**, re-run the smallest RED/GREEN proof plus the affected controls, then re-review the fix delta. Never restart the implementation to escape convergence pressure. At the cap, classify before acting: converging ⇒ fold and disclose the extension in one line; open-class ⇒ park the slice and escalate to spec §12.

- [ ] **Step 17: Re-run all closeout gates after the final fix commit.** Earlier GREEN does not transfer across a fix wave.

- [ ] **Step 18: Commit closeout docs.**

```bash
git add docs/superpowers/handoffs docs/analysis/prism-post-plan-roadmap.md
git commit -m "test(item2): record reaching-definitions closeout" \
  -m "Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>" \
  -m "Claude-Session: https://claude.ai/code/session_015U8HwBTAFzFzqJbbq82JBT"
```

- [ ] **Step 19: Push only after controller authorization.** Open one PR against `shoedog/prism` main; disclose the 55→56 invalidation, the Task 0b behaviour commit and its Tier-A evidence, and the nominal byte controls; attach the full gate table and the review convergence record. **Do not merge.**

---

## Spec-defect dispositions — nothing open

The seven defects plan v2 raised were ruled in **spec v5**; the five sol r1 findings were ruled in **spec v6** (both recorded in spec §13 and `~/code/tools/LEDGER.md`). Kept so the handoff can show each was disposed of rather than dropped.

| # | Defect raised by plan v2 | Ruled in | Disposition |
|---|---|---|---|
| 1 | Gate 9 unsatisfiable — Terraform has 0 DataFlow edges; Java/Lua unmeasured; §7.1's table covers only py/go/rs/js/ts | v5, refined v6 | Gate 9 scoped to the five §7.1 fixture languages; every other language **recorded** with its census edge count; Terraform is `gate 9 N/A — no DataFlow`; c/cpp/bash/tsx get §7.3 parity with poles as a follow-up; **Java/Lua measured in Task 0 Step 3b**. A pole is never fabricated. Sol ruled this **SOUND**. |
| 2 | §4.1's `delta_slice` "diffs by value" is false at `bffb847` | v5 | Argument corrected; the decision (leave `FlowEdge` alone) unchanged, resting on the verified synthetic-construction ground. Task 1 Step 8 carries the note for reviewers. |
| 3 | Gate 4's baseline and control binary are pre-Phase-0 | v5, refined v6 | Base is `bffb847`; **Task 0 Step 3** measures it (prior controller figure `3810/0/1` over 29 binaries + 2 doc-tests); **Task 0 Step 2** builds the control binary from the detached worktree; **Task 0b Step 9** rebuilds it at the behaviour-commit head. |
| 4 | §2.1 says "9 `==`/`!=` sites", lists eleven | v5 | Count corrected; **Task 1 Step 1** re-derives the census from `grep`, not from the spec's totals. |
| 5 | §7.4 leaves the `labels` insert rule for duplicate keys unstated | v5 | `FlowConfidence::worst()` on insert (**Task 3** Interfaces). Sol ruled this **SOUND**: duplicate observations must conservatively meet rather than take construction order. |
| 6 | §5.2's "Phase-0 goldens" are not files | v5 | Redefined as same-base binary output captured at control time (Global Constraints → Byte control). |
| 7 | Task 0b needs §9-forbidden paths | v5 | Explicit **Task-0b-only** amendment to §9, stated in Global Constraints and folded into the spec. |

| # | Sol r1 finding | Ruled in | Disposition |
|---|---|---|---|
| W1 | Twelve of the sixteen named algorithms emit no `SliceFinding` | v6 §7.6 / gate 11 | Manifest reduced to `provenance`, `taint`, `membrane`, `echo`; traversal-only algorithms covered by labeled-walk unit tests + §7.3 parity (**Task 4** Steps 2, 2b; **Task 7** Step 12). |
| W2 | Task 0b's fixture-pack ≥90 % metric is already green before the change | v6 §10 Task 0b | Corpus-level per-language thresholds, a [75 %, 90 %) attributed-residual band, a <75 % STOP (**Task 0b** Step 6). |
| W3 | The 4,097-statement cap fixture can pass through the def cap | v6 §7.2 | Three decoupled fixtures + a cap-specific `Unavailable` reason (**Task 2** Interfaces, Step 4). |
| W4 | `git checkout bffb847 -- .` stages the deletion of `examples/dfg_census.rs` | v6 gate 4 | Both control binaries built in the detached worktree `~/code/slicing-phase0-review`; no `\|\| true` (**Task 0** Step 2, **Task 0b** Step 9). |
| S1 | Tasks 0/1/2/4 recorded only compile-time "feature absent" | v6 §10 "RED shape" | Two-part RED rule in Global Constraints plus a conservative-stub step in each of those four tasks (**Steps 5b / 4b / 6 part 2 / 3b**). Task 7 exempt. |

### One measurement caveat carried forward — disclosed, not a defect

Task 0b's corpus-level threshold for **javascript** runs over **36 functions / 301 DataFlow edges** across three repos: `eval/corpora.toml`'s eleven anchors are 3 Rust + 5 Go + 3 Python, and the JS/TS/TSX edges come from vendored files inside prometheus (387 tsx / 302 ts functions), ruff (223 / 34) and prism (2 / 40). Typescript has 336 functions / 2,330 edges; tsx 612 / 4,396. The percentages are real and measurable — and far better than the fixture-pack metric sol rejected — but a three-figure edge population is a weak signal beside Rust's 247,716. **Task 0b Step 6 therefore requires the function and edge counts to be reported next to each percentage**, so the controller's [75 %, 90 %) ruling is made with the denominator in view.

### The original plan-v2 statements, kept for the record

1. **Gate 9 is unsatisfiable for at least one language.** §8 gate 9 requires "every supported language's fixture pack … has a non-empty Exact case and a non-empty NameOnly case", and §7.1 extends coverage to all twelve `Language::all()` values. The census measures **Terraform at 0 CFG nodes across 7/7 functions (0.0 % line coverage) and 0 DFG edges** (REPORT.md §G, §H) — there is no Terraform DataFlow edge to label, so no Exact pole can exist at any cap. **Java and Lua have no measured row at all** in §G/§H. The §7.1 fixture table itself assigns rows only to py/go/rs/js/ts. Proposed amendment for the owner: restate gate 9 as "both poles wherever the language produces a DFG edge, and an explicit recorded zero-edge finding otherwise". Task 6 Step 10 reports the affected languages rather than fabricating a pole.
2. **§4.1's `delta_slice` justification is false at `bffb847`** (`src/algorithms/delta_slice.rs:41-68` diffs a 4-tuple projection, not `FlowEdge` by value). The decision to leave `FlowEdge` alone survives on the verified synthetic-construction ground; only the spec's argument needs correcting in v5.
3. **§8 gate 4's baseline and the on-disk control binary are pre-Phase-0** (`3543/0/1` from `logs/baseline-c220525.log`; `bin/prism-base-c220525`). Resolved by measurement in Task 0; spec v5 should carry the `bffb847` figures.
4. **§2.1 says "9 `==`/`!=` sites" and lists eleven.** The enumeration is right; the count is wrong; the shape buckets do not partition the 35 either way. Task 1 re-derives from `grep`.
5. **§7.4 does not state the `labels` insert rule for a duplicate `(from, to)` key.** This plan prescribes `worst()`; spec v5 should say so.
6. **§5.2's "Phase-0 goldens" do not exist as files.** Resolved here as same-base binary output; spec v5 should say so.
7. **Task 0b needs §9 forbidden paths** (`src/languages/**`, `src/ast.rs` statement collection). Recorded above as an explicit Task-0b-only amendment; spec v5 must fold it.

---

## Self-review

### 1. Spec coverage — every §3/§4/§7/§8 requirement → task

| Spec requirement | Task |
|---|---|
| §3 goal 1 — every DataFlow edge carries a `FlowConfidence` from RD | 1 (payload), 2 (pass), 3 (wiring) |
| §3 goal 2 — finding confidence = worst over traversed evidence; `NameOnly` producible | 4 |
| §3 goal 3 — `--min-confidence` filters json/review/sarif/targets, default byte-identical | 5 |
| §3 goal 4 — one cache transition 55→56 | 1 (bump), 3 (persistence), 6 (verify), 7 (gate 8) |
| §3 non-goal 1 — label-only, zero edge-set change | 1 (§7.3 parity), 3 (parity upgrade), every byte control |
| §3 non-goals 2-6 — no SCIP, no boundary nodes, no per-byte identity, no `precise`/`auto` | Global Constraints; 5 (clap rejects `precise`/`auto`) |
| §4.1 `FlowConfidence`/`FlowDoubt`/`worst`/`From<ResolutionConfidence>` | 1 |
| §4.1 single-source label store `DataFlowGraph.labels`; `FlowEdge` unchanged | 3 (store), 1 Step 8 (`FlowEdge`) |
| §4.1 no `#[serde(skip)]` on the persisted payload; mandatory version bump | 1 Step 9 |
| §4.2 RD pass, ENTRY, worklist, caps, precedence | 2 |
| §4.2 signatures proposed by implementer, approved before RED (B2) | 2 Step 1 |
| §4.2 provisional capture rule (B8 i) | 2 Step 5, 3 Step 2, 6 Step 10 |
| §4.2 CFG safety joins (try-header, Go defer, branch arms) | 0 (arm provenance), 2 Step 8, 3 Step 2 |
| §4.3 labeling rule table + precedence | 2 Step 8, 3 |
| §4.4 `EvidencePath`/`EvidenceHop`/`classify_with_evidence`; `classify` defined through it | 4 |
| §4.4 index-aligned `Vec<Option<EvidencePath>>` transport; `SliceFinding` unchanged | 4 Step 6 |
| §4.4 per-algorithm evidence (echo, membrane, provenance, taint, chop, leftflow, fullflow) | 4 Step 2 |
| §4.5 `--min-confidence` / `--resolution` grammar; clap `ValueEnum`s (B3) | 5 |
| §4.5 runtime `ResolutionMode`; `RESOLUTION_MODE` constant deleted | 5 |
| §4.5 nav wire shape unchanged | 1 Step 6, 6 Interfaces |
| §4.6 cache 56, sidecar 24 | 1, 6 Step 12, 7 Step 11 |
| §4.6 PartialHit label survival analysis (B5) | "PartialHit label survival — analysis"; implemented in 3; acceptance 3 Step 11 / 7 Step 7 |
| §4.7 `DfgLabelStats`, `call-stats` additive key, `nav dfg-stats --edges` | 6 |
| §5.1 byte-control script + zero diffs | 1 Step 10, and every task's control step |
| §5.2 nominal SARIF/targets vs same-base | 5 Step 10, 7 Step 6 |
| §5.3 per-binary cache-decision control | 7 Step 6 (Global Constraints) |
| §5.4 eleven-anchor call-stats control | 7 Step 9 |
| §5.5 Tier-A matrix + quick | 0b Step 7, 6 Step 9, 7 Step 8 |
| §5 behaviour-commit rule (E4) | Global Constraints; **Task 0b** |
| §6 failure directions 1-10 | 2 Step 8 (1-5, 8), 4 Step 1 (6, 9), 5 (7), 3 Step 8 (10) |
| §7.1 fixtures + matrix probe; staged RED / GREEN split (B6) | 3 (staged RED), 6 (GREEN) |
| §7.2 unit tests — reaching, flow_confidence, finding_confidence, dfg_test | 2, 1, 4, 3 |
| §7.2 three decoupled cap tests with cap-specific reasons (sol r1 W3) | 2 Interfaces + Step 4 |
| §7.3 plumb-through parity | 1 Step 3, 3 Step 9 |
| §7.4 single-source parity | 3 Step 10 |
| §7.5 cache lifecycle + CLI | 3 Step 11, 5 |
| §7.6 per-path end-to-end — the **four** finding producers (`provenance` ×3, `taint`, `membrane`, `echo` ×2 each = 9 cases); three provenance cases (B7); four-producer manifest (sol r1 W1) | 4 Steps 2, 7, 8 |
| §7.6 traversal-only algorithms — labeled-walk unit tests, no finding minted (sol r1 W1) | 4 Step 2b (+ §7.3 parity in 1 Step 3 / 3 Step 9) |
| §8 gate 4 — base totals measured at `bffb847`; control binary built in the detached worktree (sol r1 W4) | 0 Steps 2, 3; 0b Step 9; 7 Step 5 |
| §8 gate 9 — five fixture languages need both poles; every other language recorded with its census edge count; Java/Lua measured | 0 Step 3b; 6 Step 10 |
| §8 gate 11 — delivery for the four producers only; traversal-only dispositions recorded | 4 Steps 7-8; 7 Step 12 |
| §8 gates 1-11 (all) | 7 |
| §10 "RED shape" — two-part RED for every new semantic contract (sol r1 S1), each stub closed over every symbol its command exercises (sol r2 W1) | Global Constraints; **0** Step 5b, **1** Steps 4b + 4c, **2** Step 6 part 2, **4** Steps 3b + 3c — six RED records, each with a symbol-closure table |
| §4.4 labeled twins deliver real labels, not empty or fabricated ones (sol r2 W2) | 4 Step 2b (known-label `MIXED` / `PURE` fixtures; exact endpoints and worst values) |
| §10 Task 0b middle band — only a recorded `ACCEPT` passes (sol r2 S1) | 0b Step 6 pass rule |
| §9 permitted/forbidden files | Global Constraints (+ the Task-0b-only amendment) |
| §10 sequencing, review cap 2 | Tasks 0-7; Global Constraints |
| §11 risks 1-7 | 3 Step 2 (1), 3 Step 10 (2), 2 Step 7 + REPORT.md (3), 1 Step 7 (4), 6 Step 11 (5), 7 Step 19 (6), 0 (7) |
| §12 Q1 caps from the measurement pass (A1) | 2 Interfaces |
| **Controller ruling: CFG statement-universe completeness** | **0b** |

Gaps found and closed while writing this table: §5.3 (per-binary cache-decision control) had no home in plan v1 — folded into Global Constraints and Task 7 Step 6. §11 risk 5 (the default `nameonly` admits `unlabeled`) had no documentation step — folded into Task 6 Step 11.

Gaps found and closed in v3: spec v6 gate 9 says Java and Lua are "measured in Task 0", but §10's Task 0 row does not mention it — folded in as **Task 0 Step 3b** and cross-referenced from Task 6 Step 10. Spec v6 §10 Task 4's row still names "evidence recording in provenance / taint / chop / leftflow / fullflow / echo / membrane", which is consistent with W1: *evidence recording* for the traversal-only three stays in Task 4 (Step 2b); only *finding* pairs are restricted to the four producers.

### 2. Placeholder scan

Searched for `TBD`, `TODO`, "implement later", "add appropriate…", "handle edge cases", "similar to Task N", and code steps without code. **Two intentional, ruled exceptions:**

1. **Task 2 Step 1's `// PROPOSAL` block**, which the controller must replace before RED. That is owner B2 ("implementer proposes, controller approves before RED"), and the step forbids proceeding on a guessed signature — a gate, not a placeholder.
2. **Task 0b Step 6's `_fill_` column**, which is a measurement to be taken, not a decision to be deferred: the pass rule (≥90 / [75,90) attributed / <75 STOP), the command, the metric definition, the row set, and the population counts are all fixed. Writing a number there now would be fabrication.

Everything else names a real symbol at a verified line. The four new stub blocks (Tasks 0, 1, 2, 4) are complete compilable code with their expected failing assertions spelled out, not sketches.

### 3. Type and name consistency

Checked across tasks. Corrections made against plan v1:

- `SliceResult.evidence` is `Vec<Option<EvidencePath>>` everywhere (spec v4 §4.4 / review record #9). Plan v1 Task 4 said `Vec<EvidencePath>` — a `None` must be distinguishable from empty AST-only evidence.
- `--resolution` default is **`nominal`** everywhere (spec v4 §4.5). Plan v1 Task 5's Interfaces said "default: scoped for slice/targets/sarif".
- `RD_MAX_DEFS = 2048` / `RD_MAX_LINES = 4096` everywhere, with the operand fixed as `stmt_lines.len()`. Plan v1 carried `4096`/`8192` as placeholders.
- `MinConfidence` / `ResolutionMode` are clap `ValueEnum`s (B3); plan v1 left the type unruled.
- Plan v1's Global Constraints said "Stable alias equivalence may participate in KILL" — that contradicts spec v4 §4.2 line 14 and §4.3 ("**any** alias-derived edge in v1, whether or not its flow-insensitive base is re-assigned"). v2 states the spec's rule: v1 never uses an alias relation as a kill proof, and every alias-derived edge is `NameOnly(AliasUnstable)`.
- `FlowConfidence::worst` / `is_exact` / `level` are the only three methods, spelled identically in Tasks 1, 2, 3, 4.
- `DataFlowGraph.labels` / `rd_function_stats` / `RdFileStats` are spelled identically in the analysis section and Tasks 3 and 6.
- `build_cfg_edges_with_arms` / `ArmProvenance::crosses_lexical_arm` are spelled identically in Tasks 0 and 2.
- `backward_reachable_labeled` / `dfg_forward_reachable_labeled` are spelled identically in Task 4's Interfaces and Steps.
- Commit trailer is `Co-Authored-By: Claude Fable 5.1 <noreply@anthropic.com>` in every task (plan v1 used a `<Model>` placeholder).

Corrections made in v3 (sol r1):

- The **finding-producer manifest is the four** — `provenance`, `taint`, `membrane`, `echo` — spelled identically in Task 4's header note, Task 4 Steps 2/7/8, Task 7 Step 12, and the coverage table. v2 named `chop`, `leftflow` and `fullflow` as finding producers in Task 4 Step 7; they are not, and are now covered by Step 2b's labeled-walk tests.
- **`RdOutcome::Unavailable` carries a cap-specific reason** with two distinguishable values, spelled the same way in Task 2's Interfaces, Step 4's three fixtures, and Step 6's stub expectations. v2's `Unavailable(OverCap)` was a single value and could not discriminate the two caps.
- **`~/code/tools/bin/prism-base-bffb847`** and **`~/code/tools/bin/prism-item2-0b`** are the only two control binaries, both built in `/Users/wesleyjinks/code/slicing-phase0-review`, spelled identically in the custody table, Global Constraints, Task 0 Step 2, Task 0b Step 9, and Tasks 1-7's control steps.
- The **RED-shape rule** names its two parts identically wherever it is applied: "feature absence" (recorded, not the RED) and "assertion-level failure against a conservative stub" (the RED).
- Task 0b's acceptance metric is **corpus-level** in the task, the coverage table, and the disposition record; no "fixture-pack ≥90 %" wording remains anywhere in the plan.

Corrections made in v4 (sol r2):

- **`build_cfg_edges`** is the real seam everywhere. v3's Task 0 stub called `build_cfg_edges_impl`, which does not exist at `bffb847` — `src/cfg.rs:29` exposes `build_cfg_edges`, and the stub now wraps it (Step 6 later inverts the direction so there is one walk).
- **Six RED records, not four**, because two tasks carry two semantic contracts each: Task 1 splits into the lattice (4b) and label-insensitive selection (4c); Task 4 splits into classification (3b) and transport (3c). Each record names its own command, its own stub, and its own expected failing assertion.
- **Every stub carries a symbol-closure table** listing what its RED command exercises and whether the stub or the existing tree provides it — the check sol's W1 was asking for. `From<ResolutionConfidence>` and the `src/cpg.rs` module registration are in Task 1's stub; `backward_reachable_labeled`, `EvidencePath::from_trace`, `SliceResult.evidence` and `ReviewRun.evidence` are in Task 4's transport stub.
- **`SliceResult` is not `#[non_exhaustive]` today but `ReviewRun` and `ReviewOutcome` already are** (`src/slice.rs:299`; `src/api/run.rs:288, 342`). Task 4's transport stub states the consequence: adding the field to `SliceResult` also means updating `SliceResult::new` (`src/slice.rs:314`) and every struct-literal construction.
- **Labeled-walk assertions name exact values.** v3's `hops.iter().all(...)` passed on an empty vector and its "non-empty map" admitted fabricated `Exact` labels. Task 4 Step 2b now uses two fixtures whose labels follow from §4.3 by construction (`MIXED` → one `Exact` and one `NameOnly(Killed { kill_line: 4 })`; `PURE` → one `Exact`) and asserts set equality, exact hop endpoints in order, exact per-target confidences, and the exact worst value — for both twins and for chop's fold, including the negative that an edge with one endpoint off-path does not contribute.
- **`ACCEPT` vs `SECOND PASS`** are distinguished in Task 0b Step 6, the coverage table, and the fix-round table: recording *a ruling* is the reporting obligation, recording `ACCEPT` is the pass condition.
