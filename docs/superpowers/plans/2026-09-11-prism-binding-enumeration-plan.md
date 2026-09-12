# Prism Binding-Construct Enumeration Implementation Plan

> **v6 (2026-09-12) — sol r5 folded by the controller: FIX W=5 S=1, every finding a defect in the controller's r4 fold text (remote name in Step 6; branch-aware Java span; adapter built against both clones instead of a base-binary mode; cap equality at `RD_MAX_LINES+1` accepted / `+2` refused; M6 fields `meta.admitted` / `meta.baseline_invalid` / `criteria.both_baseline_valid`). Series 12 → 7 → 9 → 5 → 5. **SLICE 1 APPROVED by the owner 2026-09-12 ("1 - ok"); Task 0 grounding started the same day; astra task-reviews required on 6a/6b/7/8/9.** Supersedes v5. .  (r2), v2 (r1), v1. Grounded on prism `origin/main` `afc78147` via the detached read-only worktree `~/code/slicing-enum-review`; every `file:line` below was read there.

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the hand-written binding-scope `match` and amendment 1c's heuristic collector with a generated, curated, digest-pinned table of binding constructs per language, and make the fail-closed default (`Uncertain`) and the non-`DefSite` binders (`Killed@binder` masks / reuse transfers) sound under the may-reaching contract — label-only, nominal bytes unchanged.

**Architecture:** Two static row tables (`BindingRow`, `CaptureRow`) keyed by `(language, kind, variant)` replace `binding_scope_rule` (`src/cpg/reaching/scope.rs:410-474`) and demote `derive_introducing_fields` (`scope.rs:672-715`) to a lint. The reaching pass (`src/cpg/reaching.rs`) gains one `FlowDoubt` variant (`OwnershipUncertain`, one `CACHE_VERSION` transition) and the spec's barrier rules: rule 1 demotes uses inside an uncertain binder's visibility span; rule 2a masks by binding identity inside a classified mask's span; rule 2b applies a reuse binder's transfer on the CFG edges that enter its body (a synthetic node per body-entry edge, never at the header), so `classify_by_reaching_sets` decides `Killed` only when no route survives. Every row carries a regression whose snippet is proven to contain the row's kind.

**Tech Stack:** Rust 2021, tree-sitter grammars pinned in `Cargo.toml`, serde_json (already used at `scope.rs:673`), a sha256 digest (Task 0 checks whether `sha2` is a dependency; else the helper prism already links), the Phase 0 byte-control scripts, the Tier-A matrix via `uv` (`eval/README.md:78-83`). No new crate dependency without a Task 0 note.

**Spec:** `/Users/wesleyjinks/code/tools/specs/2026-09-11-prism-binding-enumeration-spec.md` v0.5 — binding authority; the plan owns placement and layout, never rulings. Parent: `specs/2026-09-04-prism-item2-dataflow-confidence-spec.md` v6.5.6. Owner decisions: `DECISIONS.md` B-open, B8, E4; spec §10 Q1 (C-family depth, default scopes + unambiguous declarations), Q2 (grammar bump fails CI, default fail), Q3 (E5 closeout waits for the Tier-A re-anchor, default wait). **Defaults apply throughout.**

---

## Custody — the base this plan is anchored to

| Fact | Value |
|---|---|
| Read-only analysis tree | `/Users/wesleyjinks/code/slicing-enum-review`, detached at `afc78147`; item 2 merge `71688dc7` is an ancestor |
| Implementation clone (Task 0; an independent `git clone` of `git@github.com:shoedog/prism.git`, remote `github` — never a worktree) | `/Users/wesleyjinks/code/slicing-enum`, branch `binding-enumeration`, from `github/main` at dispatch (full SHA recorded in `custody.txt`) |
| Control binary (Task 0) | built in a **separate writable detached checkout** `~/code/slicing-enum-base` (never the review worktree, which must stay clean — no `target/`; no target-dir override exists in `.cargo/` or `Cargo.toml`), copied to `/Users/wesleyjinks/code/tools/bin/prism-base-<sha7>`; sha256 + `--version` recorded |
| Cache versions at afc78147 | `CACHE_VERSION = 85` (`src/cpg_cache.rs:201`, pin test `:741`), `NAV_CALL_EDGE_CACHE_VERSION = 46` (`src/navigation/call_edge_cache.rs:90`) — transition is **current → current+1 at dispatch/rebase**, sidecar untouched |
| Files over the 600-line cap today | `scope.rs` 893, `reaching.rs` 604 (Task 1 splits both — v4 moved the splits first; earlier review-record rows that say Task 5 describe the pre-v4 numbering), `src/languages/mod.rs` 1750 (untouched) |
| Regression fixtures | 22 capability names / 57 `eval/fixtures/<lang>/dfg_reaching_*` directories (`expected.toml`, `probe = "dfg"`, `[[expect.edges]] from/to/confidence/doubt`) — Task 0 recounts |
| Unit regression helpers | `src/cpg/reaching/tests.rs:118-165`: `parsed`, `function`, `collect_defs`, `edge`, `run`, `assert_label`; submodules `captures/cfg_joins/limits/review_regressions.rs` (176/111/187/571 lines) |
| `worst()` call sites | `src/data_flow.rs:212`, `src/cpg/reaching.rs:248`, `src/cpg/query.rs:719`; order pinned by `flow_confidence.rs:137-152` (`doubt_badness_order_is_pinned`) |
| DFG confidence fold, public path | `query.rs:719` in `dfg_forward_reachable_with_evidence` (`:665`) ← `dfg_forward_reachable_labeled` (`:657`) ← `taint_forward_labeled` (`:837`, at `:858`) / `taint_forward_cfg_labeled` (`cfg_queries.rs:205`, at `:245`) ← `src/algorithms/taint.rs:11055` — i.e. `prism --algorithm taint --taint-source <file:line> …`; `nav callers\|callees` traverse **call** edges and take no `--resolution` |
| Loop CFG shape (`src/cfg.rs:196-238`, `:408-440`) | header line → first body line (sequential fall-through, `:214-229`); last body line → header (back-edge, `:417-425`); header → first statement after the loop (exit, `:431-437`). No separate body-entry node exists. |

---

## Global Constraints

- **Label-only.** Zero DataFlow edges added/removed/re-endpointed; CFG, finding set and every nominal-mode byte unchanged (spec §1). Byte controls run branch vs base binary in the same worktree; zero differing invocations or STOP; re-anchor the invocation count if `main` added fixtures and say so.
- **Scoped surfaces may change** (`--resolution scoped`, `--min-confidence exact`, `nav dfg-stats`) — gated per task by `cd eval && uv run tier-a --matrix-only --allow-stale-sut` (expected `ok` = recorded count + this task's fixtures, `0 gap / 0 fail`) plus the task's regressions. Every moved label is listed in the task report with its row (census `--label-delta`).
- **Symmetry (owner B8).** No path raises a label to `Exact` without a per-construct regression; every downgrade case pairs with a preservation case that must stay `exact`.
- **One cache transition on the branch** (Task 7); later payload additions under the same transition amend its history line (item 2 precedent), never a second bump; gates run with `--no-cache`.
- **Files under 600 lines** (prism `CLAUDE.md:291-295`): the cap test from Task 1 enumerates every `.rs` under `src/cpg/reaching/` **dynamically** (recursive `read_dir`) plus `src/cpg/reaching.rs`, and runs in **every** touching task's gate.
- **Gates per task:** `cargo fmt --check`; clippy delta 0 vs base (`cargo clippy --all-targets --all-features -- -D warnings 2>&1 | sort`, diffed against the base run in the same worktree); the task's focused selectors; suite totals at checkpoints via `awk` over the complete unit **and** doctest logs (Task 0 Step 4).
- **RED custody.** Assertion-level RED (never a compile error: scaffold the interface first) saved on the pre-task revision to `/Users/wesleyjinks/code/tools/logs/binding-enumeration/<task>-red.log`. **One custody file, one literal absolute path:** `CUSTODY=/Users/wesleyjinks/code/tools/logs/binding-enumeration/custody.txt` — every custody redirect uses `$CUSTODY`; nothing is ever written inside the review worktree.
- **Never write to `~/code/slicing`.** Implementers work in `~/code/slicing-enum` (waves: `~/code/slicing-enum-<wave>`); codex seats get a detached review worktree as cwd.
- **Push / PR need explicit per-item owner authorization.**
- **Seats.** Implementer: codex gpt-5.6-sol (agent) or Opus for multi-file tasks; task review: **gpt-6-astra for Tasks 6a, 6b, 7, 8, 9** (reporting algebra, gate validity, span lifetime, binding identity, solver integration — astra r3 requirement), Opus for the lossless ports and curation tasks (package = `git diff <pre>..<post>` + brief + report); scoped re-review: Sonnet; checkpoints and E5: gpt-6-astra. **Briefs for 6a–9 carry the semantic partitions explicitly: nested mask, reuse within a mask, bypass, initializer/iterable preservation** (astra S1). Cap 2 fix rounds per task; at the cap classify (converging ⇒ fold + one disclosed extension; open-class ⇒ park).

---

## Slices, waves, estimates

One PR (one cache transition), four owner-approval checkpoints. **Per-task arithmetic** derives from the item 2 record — implementer time from dispatch to report, fix rounds from the SDD ledger (`records/prism-item2-plan/progress.md:23-30` Task 0: 28 min + 1 fix round 29 min + review/re-review; `LEDGER.md` rows 194/204: Task 1 mechanical port 48 min, 0 fix rounds; rows 186-190: Task 0 wall 70 min; row 209 + `progress.md:84-90`: Task 5 one fix round, Task 6 STOP → adjudication → split 6a/6b, four rounds). Seat time counts the implementer only; reviews overlap wall time and are counted as rounds.

| Task class | Seat time (h) | Fix rounds | Precedent |
|---|---|---|---|
| Split / lossless port (Tasks 1–6) | 0.5–1.0 | 0–1 | item 2 Task 1: 48 min, 0 rounds |
| Gate/census task (6b) | 1.5–2.0 | 1 | item 2 Task 0: 28 + 29 min |
| Barrier task (7, 8, 9) | 1.5–2.5 | 1–2 | item 2 Task 2: 88 min + astra review; Task 6: 4 rounds when the matrix disagreed |
| Curation task (11–27) | 0.75–1.5 | 1 | item 2 Task 5: one round |
| Integration / checkpoint (10, 19i, 27i) | controller 1.0 + gate wall (full suite ≈ 20 min, byte controls ≈ 10 min, matrix ≈ 1 min after build) | — | item 2 Task 7 |

| Slice | Tasks | Seat-hours (sum of the class ranges) | Fix rounds | Stop condition |
|---|---|---|---|---|
| **1 — core** | 0; 1 (splits); 2–5 (ports); 6 (rows); 6a; 6b; 7; 8; 9; 10 | Task 0 0.5; split 0.5–1; ports 4×0.5–1 = 2–4; 6: 0.5–1; 6a 0.5–1; 6b 1.5–2; 7–9 4.5–7.5 → **10–17** | 5–12 | nominal byte diff; matrix below recorded ok; a port needing a semantic change; an E0c RED that is not RED on main; cap 2 with open-class findings |
| **2 — curation A** | waves A–E (11–19), 19i | 9 × 0.75–1.5 = **6.75–13.5**, waves overlap in wall time | 9–12 | a new `Exact` without a preservation pair; a second behaviour question inside a task (split before dispatch); provisional rows whose `revisit` closed |
| **3 — curation B** | 20a; waves F–I (20–27); 27i | 20a 0.5; 8 × 0.75–1.5 = **6.5–12.5** | 8–11 | as slice 2; C++ template/macro semantics never guessed |
| **4 — closeout** | 28 | 2 + gate wall | 1 (astra) | blocked until the Tier-A minimum slice M6 pair exists or the owner amends E4 (Q3) |

**Waves (S4).** Slice 1 is sequential (each task touches `scope.rs`/`reaching.rs`); **Task 1 performs the mechanical splits first so the ≤ 600-line gate is satisfiable at the first touching task** (astra W8). Slice 2: **A** = 11, 12 (`binding_table/python.rs`, `tests/enumeration/python.rs`); **B** = 13 (`go.rs`); **C** = 14, 15 (`rust.rs`); **D** = 16, 17, 18 (`javascript.rs`, `tests/enumeration/{javascript,typescript}.rs`); **E** = 19 (`capture_rows.rs`, `capture.rs`, `tests/enumeration/captures.rs`). Each wave runs in its own worktree `~/code/slicing-enum-<wave>` on branch `binding-enumeration-<wave>` from the slice-1 head; tasks inside a wave are sequential. **Every wave's test module is created empty and registered in `tests/enumeration/mod.rs` by Task 6b (W7), so pre-integration selectors select real tests and no wave ever edits `mod.rs`; only the integration tasks edit the `closed` registry.** Curation tasks add cases to their own language module and fixtures under their own language directory. Integration order: A → B → C → D → E, each a `git merge --no-ff` into `binding-enumeration` followed by the full gate set (Task 19i). Slice 3: Task 20a splits `other.rs` first; waves **F** = 20, 21 (`java.rs`), **G** = 22 (`c.rs`), **H** = 23, 24, 25 (`cpp.rs`), **I** = 26, 27 (`scripting.rs`); order F → G → H → I (Task 27i).

---

## File structure (the spec left layout to the plan)

| Path | Responsibility | Task |
|---|---|---|
| `src/cpg/reaching/binding_table.rs` (new, ≤ 400) | Schema types, `rows`, `capture_rows`, `select_binding_row`, `select_capture_row`, `predicate_matches`, `grammar_digest`, `pinned_digest`, `census` | 1, 5, 6, 19 |
| `binding_table/{python,javascript,go,rust,other}.rs` (new, each ≤ 600) | Row data (`ROWS`, `DIGEST`); `other.rs` = parameter rows for Java/C/C++/Lua/Terraform/Bash until Task 20a splits it into `java.rs`, `c.rs`, `cpp.rs`, `scripting.rs` | 1–5, 20a, 11–27 |
| `binding_table/capture_rows.rs` (new) | `CaptureRow` data | 5, 19 |
| `binding_table/census/<lang>.json` (generated) + `docs/superpowers/specs/binding-enumeration/census-<lang>.md` | Census twin the tests read (`include_str!`) | 6 |
| `src/cpg/reaching/grammar_lint.rs` (new, ≈ 300) | Task 1 pure move from `scope.rs`: `NodeTypeSchema`… (`:13-38`), `collect_unclassified_binding_lines` (`:476`), `collect_binding_identifiers` (`:514`), `dominance_route_lines` (`:543`), `collect_lines_after` (`:562`), `is_binding_name` (`:573`), `introduction_is_classified` (`:581`), `is_python_comprehension_target` (`:639`), `grammar_introducing_fields`…`type_can_contain_identifier` (`:659-775`) | 5 |
| `src/cpg/reaching/scope_python.rs` (new, ≈ 90) | Task 1 pure move: `collect_python_comprehension_declarations`, `collect_python_target_identifiers` (`scope.rs:776-855`) | 1 |
| `src/cpg/reaching/scope.rs` (≈ 400 after Task 1, ≈ 480 after Task 8) | `BindingFacts`, `Declaration`, `Binding`, `BindingRelation`, `declaration_seed`, `declaration_scope`, `visible_declaration`, `implicit_binding`, `function_scope`, `scope_span`, `use_byte`; rule 2a synthesis (Task 8) | 1–5, 8 |
| `src/cpg/reaching/classify.rs` (new, ≈ 260) | Task 1 pure move: `classify_edge` (`reaching.rs:314`), `classify_by_reaching_sets` (`:438`), `matching_defs` (`:486`), `lowest_reachable_kill` (`:505`) | 1, 7, 8, 9 |
| `src/cpg/reaching/transfer.rs` (new, ≈ 160) | Rule 2b: reuse-binder discovery **before** `BindingFacts::new`, synthetic `DefSite`s with binding identities, synthetic body-entry nodes, header-use ambiguity carrier, cap re-checks | 9 |
| `src/cpg/reaching.rs` (≈ 380 after Task 1) | `reaching_definitions` (`:110`), `solve_reaching_sets` (`:258`), caps, `RdFileStats`, `RdResult`, `is_incomplete_join` | 5, 9, 19 |
| `src/cpg/flow_confidence.rs`, `src/cpg/build.rs:36-66,:69-80`, `src/navigation/queries.rs:45-52`, `src/cpg_cache.rs:199-201,:741`, `src/data_flow.rs:596`, `src/cpg/query.rs:719` | Variant + order, counters, doubt string, cache transition (6a); path-level delta (6a); per-file capture-immediate accounting (19) | 6a, 19 |
| `src/cpg/reaching/tests/enumeration/{mod,case,barrier,python,javascript,typescript,go,rust,java,c,cpp,scripting,captures}.rs` | All thirteen created and registered by Task 6b (empty `CASES` where no cases exist yet); `Case`/`check_case`, the §4.3 gates, the `closed` registry (edited by 19i/27i only) | 6–9, 11–27 |
| `examples/binding_kinds_census.rs` (new) | Census, `--label-delta` (edge labels), `--path-delta` (DFG fold via a `dfg_forward_reachable_labeled` adapter + the taint CLI leg) | 6a, 6b |
| `eval/fixtures/<lang>/dfg_binding_<kind>/` (new) | Matrix fixtures only where a label moves vs main | 7–9, 11–27 |
| `docs/superpowers/specs/2026-09-04-prism-item2-dataflow-confidence-design.md` (`:534` is the 1c entry) | Amendment 1d | 28 |

---

### Task 0: Grounding — worktree, base control, custody, census dry-run

**Files:** create `~/code/slicing-enum` (worktree); outside the repo `/Users/wesleyjinks/code/tools/bin/prism-base-<sha7>`, `/Users/wesleyjinks/code/tools/logs/binding-enumeration/{custody.txt,baseline-<sha7>-unit.log,baseline-<sha7>-doc.log,byte-control-base.log}`. **Every step starts with** `CUSTODY=/Users/wesleyjinks/code/tools/logs/binding-enumeration/custody.txt; LOGS=$(dirname $CUSTODY)` and redirects only to `$CUSTODY`/`$LOGS` (W5).

- [ ] **Step 1: Independent clone (sol r4 W3 — no write to `~/code/slicing` or `~/code/slicing-enum-review`).** `git clone -q git@github.com:shoedog/prism.git ~/code/slicing-enum && cd ~/code/slicing-enum && git remote rename origin github && FULL=$(git rev-parse github/main) && BASE=${FULL:0:7} && git checkout -q -b binding-enumeration $FULL; echo "base_full=$FULL base=$BASE $(date -u +%FT%TZ)" >> $CUSTODY`. The protected review worktree is NOT checked out or fetched by this plan; Task 0 only asserts it: `test "$(git -C ~/code/slicing-enum-review rev-parse HEAD)" = "$FULL" || echo "review worktree at $(git -C ~/code/slicing-enum-review rev-parse --short HEAD) ≠ base $BASE — the controller re-detaches it manually before dispatch" >> $CUSTODY; test -z "$(git -C ~/code/slicing-enum-review status --porcelain)" && echo review_worktree_clean >> $CUSTODY`.
- [ ] **Step 2: Versions + fixture recount (S1).** Record `grep -n '^const CACHE_VERSION' src/cpg_cache.rs`, `grep -n '^const NAV_CALL_EDGE_CACHE_VERSION' src/navigation/call_edge_cache.rs`, `grep -n 'assert_eq!(super::CACHE_VERSION' src/cpg_cache.rs`, `ls -d eval/fixtures/*/dfg_reaching_* | wc -l` (expect 57) and `ls -d eval/fixtures/*/dfg_reaching_* | xargs -n1 basename | sort -u | wc -l` (expect 22) into `$CUSTODY` (`| tee -a $CUSTODY`).
- [ ] **Step 3: Control binary — built in a second independent clone, never in any protected checkout (astra W7, sol r4 W3).** `git clone -q git@github.com:shoedog/prism.git ~/code/slicing-enum-base && git -C ~/code/slicing-enum-base checkout -q --detach $FULL && cd ~/code/slicing-enum-base && cargo build --release && cp target/release/prism /Users/wesleyjinks/code/tools/bin/prism-base-$BASE && shasum -a 256 /Users/wesleyjinks/code/tools/bin/prism-base-$BASE >> $CUSTODY && /Users/wesleyjinks/code/tools/bin/prism-base-$BASE --version >> $CUSTODY` (the version string must not contain `-dirty`). No `git worktree add` anywhere in this plan; `git -C ~/code/slicing worktree list` must be unchanged before/after Task 0 (record both in `$CUSTODY`).
- [ ] **Step 4: Baseline totals incl. doctests (W1).**
```bash
cd ~/code/slicing-enum
cargo test --all-targets --all-features --no-fail-fast 2>&1 | tee $LOGS/baseline-$BASE-unit.log
cargo test --doc --all-features 2>&1 | tee $LOGS/baseline-$BASE-doc.log
cat $LOGS/baseline-$BASE-{unit,doc}.log | awk '/^test result:/{p+=$4; f+=$6; i+=$8; n++} END{printf "result_lines=%d passed=%d failed=%d ignored=%d\n",n,p,f,i}' | tee -a $CUSTODY
```
Expected `failed=0`. Then the literal byte controls, base vs base (S1), from `~/code/slicing-enum`: `cargo build --release && scripts/phase0-byte-control.sh /Users/wesleyjinks/code/tools/bin/prism-base-$BASE target/release/prism 2>&1 | tee $LOGS/byte-control-base.log; scripts/item2-byte-control.sh /Users/wesleyjinks/code/tools/bin/prism-base-$BASE target/release/prism 2>&1 | tee -a $LOGS/byte-control-base.log` → record the two population counts (item 2 closed at 1,645 and 280) and "0 differing"; `cd eval && uv run tier-a --matrix-only --allow-stale-sut` → record `ok/gap/fail` (item 2 closed at 159/0/0).
- [ ] **Step 5: Census dry run over every grammar the build actually uses (W2).**
```bash
cd ~/code/slicing-enum && for m in $(cargo metadata --format-version 1 | jq -r '.packages[] | select(.name|startswith("tree-sitter-")) | .manifest_path'); do d=$(dirname $m); for f in $(find $d -name node-types.json -not -path '*/node_modules/*'); do printf "%s %s named=%s sha=%s\n" "$(basename $d)" "${f#$d/}" "$(python3 -c "import json;print(sum(1 for t in json.load(open('$f')) if t.get('named')))")" "$(shasum -a 256 $f | cut -c1-16)"; done; done | tee -a $CUSTODY
```
Expected: 12 rows (the vendored `tree-sitter-typescript` yields `typescript/src/node-types.json` and `tsx/src/node-types.json` with distinct digests), named counts within ±5 of spec §4.1; a larger delta = a grammar moved: STOP and re-record §4.1. Also `grep -n '^sha2' Cargo.toml || grep -rln 'Sha256' src | head -3` to fix the digest helper for Task 6.
- [ ] **Step 6: Rebase discipline (sol r1 W3, sol r5 W1)**, written into `$CUSTODY`: before every checkpoint and before any push, `git -C ~/code/slicing-enum fetch -q github; NEW=$(git -C ~/code/slicing-enum rev-parse github/main); test "$NEW" = "$FULL" || { git -C ~/code/slicing-enum rebase github/main; FULL=$NEW; BASE=${FULL:0:7}; git -C ~/code/slicing-enum-base fetch -q github && git -C ~/code/slicing-enum-base checkout -q --detach $FULL; re-run Step 1's ASSERTIONS only (review-worktree SHA vs $FULL and clean — never a checkout or clone), Step 3 (rebuild the control from the refreshed base clone + hashes), Step 4 (baselines); record the new FULL; first collisions are src/cpg_cache.rs (CACHE_VERSION + history line) and src/navigation/call_edge_cache.rs; }`.
- [ ] **Step 7: Commit custody in `~/code/tools`** (`logs/binding-enumeration/`, `bin/`): `enum: Task 0 grounding — base <sha7>, control binary, baselines incl. doctests`.

---

### Task 1: Mechanical splits of `scope.rs` (893) and `reaching.rs` (604) + the dynamic cap test (astra W8)
**Behaviour question:** none — pure moves (cut/paste; only `use` lines and `pub(super)` visibility change).
**Files:** create `grammar_lint.rs` (from `scope.rs:13-38`, `:476-775`), `scope_python.rs` (from `scope.rs:776-855`), `classify.rs` (from `reaching.rs:314-523`); modify `scope.rs`, `reaching.rs` (`mod` lines), `tests.rs:462` (`mod enumeration;`), `tests/enumeration/mod.rs` (created; the cap test only).
- [ ] **Step 1: RED (assertion-level):**
```rust
#[test] fn reaching_module_files_are_under_the_cap() {   // dynamic: any new file under src/cpg/reaching/ is covered with no edit (W6)
    fn walk(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) { for e in std::fs::read_dir(dir).unwrap() { let p = e.unwrap().path();
        if p.is_dir() { walk(&p, out) } else if p.extension().is_some_and(|x| x == "rs") { out.push(p) } } }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cpg");
    let mut files = vec![root.join("reaching.rs")]; walk(&root.join("reaching"), &mut files);
    assert!(files.len() >= 6, "expected the reaching module tree, found {}", files.len());
    for f in files { let n = std::fs::read_to_string(&f).unwrap().lines().count(); assert!(n <= 600, "{}: {n} lines", f.display()); }
}
```
Run → FAIL `scope.rs: 893 lines`. Save `task1-red.log`.
- [ ] **Step 2: Moves.** Expected sizes: `scope.rs` ≈ 400, `reaching.rs` ≈ 380, `classify.rs` ≈ 260, `grammar_lint.rs` ≈ 300, `scope_python.rs` ≈ 90 (S2: `scope.rs` grows to ≈ 480 in Task 8; if any task pushes it past 560 the pre-declared next split is `declaration_seed`/`declaration_scope` (`:314-409`) into `scope_seed.rs`).
- [ ] **Step 3: GREEN** — cap test; `cargo test --lib cpg::reaching` unchanged; full unit + doc suites (awk); byte controls 0 diff (pure moves have no nominal-byte effect; the control proves it); clippy delta 0. **Step 4: Commit** `enum(split): grammar_lint.rs, scope_python.rs, classify.rs extracted (pure moves); dynamic 600-line cap test`.

### Task 2 (E0a-py): Binding-table schema + Python rows, lossless port

**Behaviour question:** none — port equivalence (scope creation, reuse, visibility, parameter seeding, fallback unchanged).

**Files:** create `binding_table.rs`, `binding_table/python.rs`, `tests/enumeration/python.rs`; modify `scope.rs` (`DeclarationKind`/`BindingScopeRule` move out; lookup rows first, then the remaining arms; delete the Python arm), `reaching.rs` (`mod binding_table;`), `tests/enumeration/mod.rs` (register `python`).

**Interfaces — Produces (used verbatim by every later task):**
```rust
pub(crate) enum Role { Scope, Binding, NotBinding }
pub(crate) struct RoleSet(u8);   // RoleSet::of(&[Role::Scope, Role::Binding]); .has(Role)
pub(crate) enum Predicate { FieldTextIs { field: &'static str, any_of: &'static [&'static str] }, OperatorIs { any_of: &'static [&'static str] },
    ParentKindIs { kind: &'static str }, HasField { field: &'static str }, IsImmediatelyInvoked }
pub(crate) enum Visibility { WholeScope, AfterIntroduction, Header, Comprehension, Custom(&'static str) }
pub(crate) enum Ruling { Classified, Uncertain { reason: &'static str, revisit: &'static str } }
pub(crate) enum Timing { Deferred, Immediate, Unknown }
pub(crate) enum DeclarationKind { Parameter, GoShort, JavaScriptVar, PythonAssignment, Reuse, Other, Pattern, CaptureCopy }
pub(crate) struct BindingRow { pub language: Language, pub kind: &'static str, pub variant: Option<Predicate>, pub roles: RoleSet,
    pub fields: &'static [&'static str], pub declaration: Option<DeclarationKind>, pub visibility: Visibility, pub ruling: Ruling, pub regression: &'static str }
pub(crate) struct CaptureRow { pub language: Language, pub kind: &'static str, pub variant: Option<Predicate>, pub timing: Timing, pub regression: &'static str }
pub(crate) fn rows(language: Language) -> &'static [BindingRow];
pub(crate) fn capture_rows(language: Language) -> &'static [CaptureRow];
pub(crate) fn select_binding_row(parsed: &ParsedFile, node: Node<'_>) -> Option<&'static BindingRow>; // predicate rows first; residual (variant None) last; else None
pub(crate) fn select_capture_row(parsed: &ParsedFile, node: Node<'_>) -> Option<&'static CaptureRow>;
pub(crate) fn predicate_matches(parsed: &ParsedFile, node: Node<'_>, p: &Predicate) -> bool;      // IsImmediatelyInvoked => false until Task 19 (comment names Task 19; no todo!)
pub(crate) fn grammar_digest(language: Language) -> String;   // hex sha256(node_types_json())
pub(crate) fn pinned_digest(language: Language) -> &'static str;
```
- [ ] **Step 1: Scaffold the module** with the types above, `rows(Python)` returning an empty slice, other languages empty; `cargo build` green.
- [ ] **Step 2: RED (assertion-level):**
```rust
#[test] fn python_rows_reproduce_the_old_match_arms() {
    let scope_kinds = ["function_definition","lambda","class_definition","list_comprehension","set_comprehension","dictionary_comprehension","generator_expression"];
    for k in scope_kinds { assert!(rows(Language::Python).iter().any(|r| r.kind == k && r.roles.has(Role::Scope)), "missing scope row {k}"); }
    for k in ["assignment","augmented_assignment","named_expression"] { let r = rows(Language::Python).iter().find(|r| r.kind == k).unwrap_or_else(|| panic!("missing {k}"));
        assert_eq!(r.declaration, Some(DeclarationKind::PythonAssignment)); assert!(r.roles.has(Role::Binding)); }
}
```
Run `cargo test --lib cpg::reaching::tests::enumeration::python` → FAIL `missing scope row function_definition`. Save `task2-red.log`.
- [ ] **Step 3: Rows** — 7 scope kinds (`function_definition` also `Binding` on field `name`), 3 assignment kinds (`Binding`, `PythonAssignment`, `AfterIntroduction`, fields `["left"]`), `regression: "e0a-py-<kind>"` (replaced by Task 6b's cases; until then strings only).
- [ ] **Step 4: Lookup** in `binding_scope_rule`: `if let Some(r) = rows(language).iter().find(|r| r.kind == kind && r.variant.is_none()) { return BindingScopeRule { creates_scope: r.roles.has(Role::Scope), declaration: r.declaration }; }` before the `match`; delete the Python arm.
- [ ] **Step 5: GREEN + equivalence:** `cargo test --lib cpg::reaching` all green unchanged; byte controls 0 diff; matrix unchanged; cap test (Task 1) green.
- [ ] **Step 6: Commit** `enum(E0a-py): binding-table schema; Python scope/declaration arms become rows (lossless port)`.

### Task 3 (E0a-js): JS/TS/TSX rows incl. the `for_in_statement` variants
**Behaviour question:** none (`scope.rs:426-441`; `for_in_statement` `let|const` header at `:335-345` and `:609-620`).
- [ ] **Step 1: RED** (module exists, so assertion-level): rows for `statement_block`, `class_body`, `function_declaration`, `function_expression`, `arrow_function`, `for_statement` (Scope); `variable_declaration` (`JavaScriptVar`); `lexical_declaration`/`class_declaration` (`Other`); **two** `for_in_statement` rows — `variant: Some(FieldTextIs{field:"kind", any_of:&["let","const"]})` with `Visibility::Header`, and residual `None` (`JavaScriptVar`); `rows(TypeScript)` and `rows(Tsx)` return the same slice. Save `task3-red.log`.
- [ ] **Step 2: Rows** in `javascript.rs`; `declaration_seed` (`:335-345`) and `introduction_is_classified` (`:609-620`) test `select_binding_row(parsed, node).is_some_and(|r| r.visibility == Visibility::Header)` instead of the literal `matches!`; delete the JS arm.
- [ ] **Step 3: GREEN**: `javascript/dfg_reaching_js_var_hoisting`, `*/dfg_reaching_shadowed_inner*`, `typescript/dfg_reaching_cfg_gap` unchanged (matrix); byte controls 0; cap check. **Step 4: Commit** `enum(E0a-js): JS/TS/TSX arms become rows; for_in_statement let/const header as a predicate row`.

### Task 4 (E0a-go) · Task 5 (E0a-rs)
Same shape: RED asserting the rows listed in `scope.rs:442-457` (Go: 7 scope kinds, the five statement kinds `Header`; `short_var_declaration` `GoShort`; `var`/`const_declaration` `Other`) and `:458-462` (Rust: `block`; `let_declaration`/`const_item`/`static_item` `Other`); rows; delete the arm; GREEN with `go/dfg_reaching_go_short_var_if`, `go/dfg_reaching_defer_argument_now*`, `rust/dfg_reaching_rust_let_shadow`, `review_regressions.rs` unchanged; byte controls 0; cap check. Commits `enum(E0a-go): …`, `enum(E0a-rs): …`.

### Task 6 (E0a-x): Parameter rows, capture residual rows, `match` deleted
**Behaviour question:** none (parameter seeding `scope.rs:318-331`, `:589-601`; capture boundaries `capture.rs:12-48`).
**Files:** create `other.rs`, `capture_rows.rs`; modify `scope.rs`, `reaching.rs`, `capture.rs` (`is_nested_callable_kind` consults `select_capture_row(..).is_some()` — unchanged behaviour because every boundary kind gets a `Deferred` residual row).
- [ ] **Step 1: RED (W4-corrected invariant):**
```rust
#[test] fn parameter_rows_exist_exactly_where_the_grammar_has_a_parameters_node() {
    let minimal = [(Language::Python,"def f(a):\n    pass\n"),(Language::JavaScript,"function f(a) {}\n"),(Language::TypeScript,"function f(a: T) {}\n"),(Language::Tsx,"function f(a: T) {}\n"),
        (Language::Go,"func f(a int) {}\n"),(Language::Java,"class C { void f(int a) {} }\n"),(Language::C,"void f(int a) {}\n"),(Language::Cpp,"void f(int a) {}\n"),
        (Language::Rust,"fn f(a: i32) {}\n"),(Language::Lua,"function f(a) end\n"),(Language::Terraform,"resource \"x\" \"y\" {}\n"),(Language::Bash,"f() { :; }\n")];
    for (lang, src) in minimal { let p = parsed(src, lang); let func = function(&p);
        let has_params = p.find_parameters_node(&func).is_some();
        let has_row = rows(lang).iter().any(|r| r.declaration == Some(DeclarationKind::Parameter));
        assert_eq!(has_params, has_row, "{lang:?}: parameters node {has_params} vs parameter row {has_row}"); }
    assert!(!rows(Language::Bash).iter().any(|r| r.declaration == Some(DeclarationKind::Parameter)));       // explicit exemptions
    assert!(!rows(Language::Terraform).iter().any(|r| r.declaration == Some(DeclarationKind::Parameter)));
}
#[test] fn every_callable_boundary_kind_has_a_residual_capture_row() { for lang in Language::all() { for kind in lang.callable_boundary_node_types() {
    assert!(capture_rows(lang).iter().any(|r| r.kind == kind && r.variant.is_none()), "{lang:?} {kind}"); } } }
```
(`find_parameters_node`, `src/ast.rs`, returns the `parameters` field or a C/C++ declarator chain; Bash `function_definition` and Terraform `block` have neither ⇒ `None`.) Run → FAIL (no parameter rows). Save `task6b-red.log`.
- [ ] **Step 2: Parameter rows** — Python `parameters`; JS/TS `formal_parameters`; Go `parameter_list`; Rust `parameters`; Java `formal_parameters`/`spread_parameter`; C/C++ `parameter_list`/`parameter_declaration`; Lua `parameters` — fields from `node-types.json`, `WholeScope`, `Classified`. Both the byte-range test and the row must agree (ruling 4).
- [ ] **Step 3: Capture residual rows** — one `Deferred` row per `callable_boundary_node_types()` kind (`src/languages/mod.rs:139-152`).
- [ ] **Step 4: Delete the `match`** (`binding_scope_rule` becomes a two-line lookup). **Step 5: GREEN** — `cargo test --lib cpg::reaching`; full unit + doc suites (awk); byte controls 0; matrix unchanged; clippy delta 0; cap test. **Step 6: Commit** `enum(E0a-x): parameter rows for all languages, capture residual rows, match deleted`.

**Port rule:** an equivalence test that can only pass by changing a row's semantics (e.g. comprehension-target visibility) is not a port — stop and record it for the owning curation task.

---

### Task 6a: Scaffold `OwnershipUncertain`, the pinned order, counters, doubt string, cache transition — and the path-level delta (W4, W2)
**Behaviour question:** none for the scaffold (rule 1 is Task 7); one for the order: does the pinned `worst()` order change any *path-level* query result, and is each such change listed and matrix-gated?
**Files:** `flow_confidence.rs:22-36,:52-59,:96-103,:137-152`; `build.rs:36-66`; `queries.rs:45-52`; `cpg_cache.rs:199-201,:741`; `examples/binding_kinds_census.rs` (`--path-delta` mode, created here, extended in Task 6); `tests/enumeration/barrier.rs` (order regressions only).
- [ ] **Step 1: Scaffold, keeping the existing relative ranks (astra W5)** — `FlowDoubt::OwnershipUncertain { construct_line: u32 }` added with a provisional `badness_key` of `(2, 0)` **above `Killed` only**, the other five keeping today's order (`Killed 1, OwnershipUncertain 2, SameLine 3, CfgIncomplete 4, AliasUnstable 5, CallNameOnly 6`); `record_label` arm + fields `dfg_label_nameonly_ownership_uncertain` and `dfg_label_capture_immediate` (the latter populated in Task 19 through `record_rd_stats`, never `record_label`); `queries.rs` string `(Some("ownership_uncertain"), Some(construct_line))`; `CACHE_VERSION` current+1 with history line `/// - vN: FlowDoubt::OwnershipUncertain payload (binding enumeration).`; pin at `:741`; `ALL` (`:96-103`) gains the variant. Rule 1 is **not** implemented; nothing produces the variant yet.
- [ ] **Step 2: RED (assertion-level):** rewrite `doubt_badness_order_is_pinned` (`:137-152`) to the ruled order `Killed < OwnershipUncertain < AliasUnstable < SameLine < CfgIncomplete < CallNameOnly` and add `every_adjacent_pair_is_ordered` (each consecutive pair both directions) — FAILS on the provisional ranks at `AliasUnstable`/`SameLine`. Save `task6a-red.log`. **Step 2b:** set the ruled ranks (`AliasUnstable 3, SameLine 4, CfgIncomplete 5`); tests GREEN.
- [ ] **Step 3: Path-level delta (sol r2 W2, astra W4, sol r4 W2, sol r5 W3).** `query.rs:719` folds `path_confidence.worst(confidence)` along successive DataFlow edges, so the re-rank can change a *query* result (`AliasUnstable` → `SameLine`/`CfgIncomplete` on a path) while no single edge label moves. `nav callers|callees` traverse call edges and take no `--resolution`, so they never reach the fold. Add `--path-delta <base-binary>` to `examples/binding_kinds_census.rs`: **(a) adapter leg** — for EVERY `eval/fixtures/<lang>/dfg_*` directory (all 57), build the CPG in-process and call `dfg_forward_reachable_labeled(&from)` (`query.rs:657` → `:665` → the fold at `:719`) from every Def `VarLocation` in the fixture, once against the branch and once against the base (sol r5 W3: the base binary has no such mode — the census example's source is copied unchanged into the base clone `~/code/slicing-enum-base` and built there, so the SAME adapter runs against both revisions' CPG code; the two executables' JSON outputs are then compared), and assert per fixture that at least one traversed path is non-empty (a fixture with zero paths is an ERROR row, never a silent skip); **(b) CLI leg** — the fixtures ship no diff files, so the census GENERATES one per fixture (`git diff $(git hash-object -t tree /dev/null) HEAD -- <fixture dir>` written under `target/path-delta/<lang>/<fixture>.diff`) and runs the one public path that reaches the fold on both binaries: `prism review --repo <fixture dir> --diff <that file> --format json` (the review path's DataFlow reachability; verify the exact subcommand and flags against `src/main.rs` at Task 0 and record them in `$CUSTODY` — if `review` does not reach `query.rs:719`, the adapter leg alone is authoritative and the CLI leg is dropped with that fact recorded). Differing outputs are listed per fixture with the path; errors are retained in the report, not swallowed.
- [ ] **Step 4: GREEN** — order tests; `cpg_cache` lifecycle tests; `nav dfg-stats` shows both counters at 0; byte controls 0; matrix unchanged (no label produces the variant yet); `--path-delta` report attached; cap check. **Step 5: Commit** `enum(scaffold): FlowDoubt::OwnershipUncertain (cache vN→vN+1), pinned six-element order with path-level delta, counters`.

---

### Task 6b (E0b): Census, digests, staged totality, `Case` corpus, wave modules, the §4.3 gates
**Behaviour question:** Does every candidate kind have exactly one selected row per occurrence, with a case that parses it?
**Files:** create `examples/binding_kinds_census.rs`, `census/<lang>.json` ×12, `docs/superpowers/specs/binding-enumeration/census-<lang>.md`, `tests/enumeration/case.rs`; modify `binding_table.rs` (`grammar_digest`, `pinned_digest`, `census`), each `binding_table/<lang>.rs` (`DIGEST`; provisional rows).
**Interfaces — Produces:**
```rust
pub(super) struct Case { pub id: &'static str, pub language: Language, pub kind: &'static str, pub variant: Option<&'static str>,
    pub src: &'static str, pub expect: &'static [(&'static str, &'static str, FlowConfidence)],
    pub expect_counter: Option<(&'static str, i64)> }                              // (DfgLabelStats field, delta vs the same snippet without the construct)
pub(super) fn nodes_of_kind<'t>(parsed: &'t ParsedFile, kind: &str) -> Vec<Node<'t>>;
pub(super) fn check_case(case: &Case);    // (1) parse; nodes_of_kind(kind).len() > 0; (2) the row selected for the first such node has regression == case.id;
                                          // (3) every expect pair via edge()+assert_label; (4) ruling assertion — Uncertain ⇒ some expect is OwnershipUncertain{..}
                                          //     (the variant exists since Task 6a); Classified mask ⇒ some Killed{..} AND some Exact; NotBinding ⇒ invariance;
                                          //     EXPLICIT EXEMPTION: a row whose ruling is Uncertain{reason:"not yet curated",..} (provisional, Step 5) asserts only (1)–(3);
                                          //     the exemption is keyed on that literal reason, so it ends when the curation task replaces the row, and
                                          //     no_provisional_row_whose_revisit_task_closed proves it cannot outlive the task (W4);
                                          // (5) expect_counter: label counters (dfg_label_*) via DfgLabelStats::record_label over the labels; RD-stat counters
                                          //     (dfg_label_capture_immediate, dfg_rd_*) via the RdFileStats the extended `run()` helper returns, summed the way
                                          //     build.rs::record_rd_stats sums them (W3) — the field name decides which accounting is read
pub(super) fn all_cases() -> Vec<&'static Case>;      // shared JS/TS/TSX rows: one Case instance PER LANGUAGE with the same id (astra W6); lookup key = (language, id)
pub(crate) struct CensusKind { pub kind: String, pub named: bool, pub heuristic_flags: Vec<String>, pub table_row: bool, pub candidate: bool, pub corpus_occurrences: usize }
pub(crate) fn census(language: Language) -> &'static Census;   // include_str! of census/<lang>.json
```
- [ ] **Step 1: Scaffold (W5, W7)** — `pinned_digest` returns `""` per language; a shared row's regression id resolves to one `Case` per language (`javascript.rs` holds the JS instance, `typescript.rs` the TS and TSX instances); `census()` parses the JSON files (initially `{"digest":"","kinds":[]}` committed by hand); `Case`/`check_case`/`all_cases`; **all thirteen `tests/enumeration/*.rs` modules created (each `pub(super) const CASES: &[Case] = &[];` where empty) and registered in `mod.rs`** — waves fill their own file and never touch `mod.rs`; `cargo build` green. `tests.rs::run` (`:118-135`) is extended to return `RdFileStats` that also carries the capture-immediate accounting once Task 19 adds it (signature unchanged).
- [ ] **Step 2: RED (assertion-level, W6-corrected gates):**
```rust
#[test] fn binding_table_digest_matches_grammar() { for l in Language::all() { assert_eq!(grammar_digest(l), pinned_digest(l), "{l:?}: grammar changed — re-run the census and re-curate"); } }
#[test] fn binding_table_is_total_over_candidates() { for l in Language::all() { let c = census(l); assert!(!c.kinds.is_empty(), "{l:?}: empty census");
    for k in c.kinds.iter().filter(|k| k.candidate || !k.heuristic_flags.is_empty()) { assert!(rows(l).iter().any(|r| r.kind == k.kind), "{l:?} {} has no row", k.kind); } } }
#[test] fn binding_table_rows_have_regressions() { let cases = all_cases(); for l in Language::all() { for r in rows(l).iter().chain_capture(capture_rows(l)) {
    let c = cases.iter().find(|c| c.id == r.regression && c.language == l).unwrap_or_else(|| panic!("{l:?} {} → missing case ({l:?}, {})", r.kind, r.regression));
    assert_eq!(c.kind, r.kind); check_case(c); } } }   // per-language lookup: a row shared by JS/TS/TSX needs three case instances (astra W6)
#[test] fn every_case_selects_exactly_one_row_per_occurrence() { for c in all_cases() { let p = parsed(c.src, c.language); let nodes = nodes_of_kind(&p, c.kind);
    assert!(!nodes.is_empty(), "{}: snippet has no {} node", c.id, c.kind);
    for n in nodes { let hits = rows(c.language).iter().filter(|r| r.kind == c.kind && r.variant.as_ref().is_some_and(|q| predicate_matches(&p, n, q))).count();
        let residual = rows(c.language).iter().any(|r| r.kind == c.kind && r.variant.is_none());
        assert!(hits == 1 || (hits == 0 && residual), "{}: {hits} predicate rows matched one occurrence", c.id); } } }
#[test] fn uncertain_rows_carry_reasons() { /* every Uncertain row: non-empty reason and revisit */ }
#[test] fn capture_table_covers_every_boundary_kind() { /* every callable_boundary_node_types() kind has a CaptureRow */ }
#[test] fn no_provisional_row_whose_revisit_task_closed() { let closed: &[&str] = &[]; /* only 19i/27i append here */
    for l in Language::all() { for r in rows(l) { if let Ruling::Uncertain{reason:"not yet curated", revisit} = r.ruling { assert!(!closed.contains(&revisit), "{l:?} {} still provisional after {revisit}", r.kind); } } } }
```
Run → FAIL at `digest` (`"" != <hex>`), `total` (empty census), `rows_have_regressions` (missing case `e0a-py-…`). Save `task6-red.log`.
- [ ] **Step 3: Census binary** (`cargo run --release --example binding_kinds_census -- --out src/cpg/reaching/binding_table/census --docs docs/superpowers/specs/binding-enumeration --corpus eval/fixtures`; `--label-delta <base-binary>` mode runs `nav dfg-stats --edges` on every `eval/fixtures/<lang>/dfg_*` with both binaries and prints differing labels). **Step 4: Digests** (TS and TSX distinct over a shared row slice). **Step 5: Provisional rows** for every candidate kind without a curated row: `Uncertain { reason: "not yet curated", revisit: "<E1a-1|E1b|…>" }`, `WholeScope`, a generated `Case` whose snippet contains the kind and whose `expect_counter` is `Some(("dfg_label_nameonly_ownership_uncertain", 0))` until Task 7 (kind parses; counter defined but zero) — each curation task replaces them.
- [ ] **Step 6: GREEN** all gates (per language); byte controls 0; matrix unchanged. **Step 7: Commit** `enum(E0b): grammar census + digests, provisional Uncertain rows, Case corpus and the totality/exclusivity/regression gates`.

---

### Task 7 (E0c-1): `OwnershipUncertain`, order, counters, cache transition, scope-bounded rule 1
**Behaviour question:** Can a use outside an uncertain binder's span ever be demoted, or a use inside it ever be `Exact`? (both no)
**Files:** `classify.rs` (`classify_edge`); `grammar_lint.rs` (`collect_unclassified_binding_lines` records the span); `tests/enumeration/barrier.rs`; `eval/fixtures/java/dfg_binding_instanceof_pattern/`. (Variant, order, counters and the cache transition already exist — Task 6a.)
- [ ] **Step 1: RED (assertion-level; the variant exists, so the failure is `Exact != OwnershipUncertain`):**
```rust
pub(super) const CASES: &[Case] = &[
  Case { id: "java-instanceof-pattern-uncertain", language: Language::Java, kind: "record_pattern", variant: None,
    src: "int f(Object o) {\n    int x = source();\n    if (o instanceof R(var x)) { sink(x); }\n    return 0;\n}\n",
    expect: &[("2:x", "3:x", FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { construct_line: 3 }))], expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)) },
  Case { id: "rust-if-let-uncertain-outside-scope", language: Language::Rust, kind: "tuple_struct_pattern", variant: None,
    src: "fn f(opt: Option<i32>) {\n    let x = source();\n    if let Some(x) = opt { observe(x); }\n    sink(x);\n}\n",
    expect: &[("2:x", "4:x", FlowConfidence::Exact)], expect_counter: None },          // span = if_expression.consequence, not `body`: after-block use preserved
  Case { id: "java-instanceof-same-line-guarded-use", language: Language::Java, kind: "record_pattern", variant: None,
    src: "int f(Object o) {\n    int x = source();\n    if (o instanceof R(var x) && sink(x)) { }\n    return x;\n}\n",
    expect: &[("2:x", "3:x", FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { construct_line: 3 })), ("2:x", "4:x", FlowConfidence::Exact)], expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)) },
  Case { id: "rust-if-let-empty-span", language: Language::Rust, kind: "tuple_struct_pattern", variant: None,
    src: "fn f(opt: Option<i32>) {\n    let x = source();\n    if let Some(x) = opt {}\n    sink(x);\n}\n",
    expect: &[("2:x", "4:x", FlowConfidence::Exact)], expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 0)) },   // Empty known span ≠ Unknown
  Case { id: "unknown-scope-fail-closed", language: Language::Bash, kind: "variable_assignment", variant: None,
    src: "f() {\n    x=$(source)\n    for x in a b; do :; done\n    sink \"$x\"\n}\n",
    expect: &[("2:x", "4:x", FlowConfidence::NameOnly(FlowDoubt::OwnershipUncertain { construct_line: 3 }))], expect_counter: Some(("dfg_label_nameonly_ownership_uncertain", 1)) },
];
```
Run `cargo test --lib cpg::reaching::tests::enumeration::barrier` → cases 1 and 3 (Java) FAIL `left: Exact, right: NameOnly(OwnershipUncertain { construct_line: 3 })`; cases 2 and 4 GREEN and must stay so (the `consequence`-span and empty-span controls); case 5 (Bash) FAILS (no row, census-only ⇒ Unknown ⇒ function span). Save `task7-red.log`.
- [ ] **Step 1b: Java pattern-guard span — branch-aware, never continuous (sol r4 W1, sol r5 W2).** Task 6b's provisional rows give `record_pattern`/`type_pattern` `WholeScope`, which would leave the Java `sink(x)` after the `if` under function-wide uncertainty and the same-line guarded use `Exact`. Task 7 assigns the Java guard binder a **set of disjoint visibility spans** in `binding_table/java.rs`, per Java flow scoping (JLS §6.3.2.2): for `o instanceof R(var x)` in the condition, the binding is visible in the condition text after the pattern and in `if_statement.consequence` only; for a negated pattern `!(o instanceof R(var x))`, in `alternative` only (and, when the then-branch cannot complete normally, in the statements after the `if`); the then-branch is NEVER inside a negated pattern's span (a continuous binder→`alternative` span would wrongly demote outer-name uses in the then-branch). Field names `consequence`/`alternative` are read from the pinned `node-types.json` (`:2148`, `:2168`). Assertions: case 1 (`java-instanceof-pattern-uncertain`) asserts BOTH the in-branch demotion AND the after-`if` `Exact` (the second assertion is what fails under `WholeScope`); case 3 asserts the same-line guarded use; a new negated case asserts then-branch preservation + else-branch demotion; `barrier.rs` asserts the span set's byte ranges explicitly.
- [ ] **Step 2: Rule 1** in `classify_edge` (moved to `classify.rs`): replace `if lookup_uncertain || construct_uncertain { flat }` with `if let Some(line) = binding_facts.uncertain_construct_on_route(&edge.to.path.base, &candidates, mapped_defs, use_node, use_line, successors) { return NameOnly(OwnershipUncertain{construct_line: line}) }` where the helper requires an unflagged `def→construct→use` route **and** `use ∈ visibility_span(construct)`. **Visibility span (astra W1):** `grammar_lint::visibility_span(parsed, binder_node) -> SpanKind::{Known(ScopeSpan), Empty, Unknown}` derives the span from the construct's **actual scope/guard fields taken from the pinned `node-types.json`**, never a hard-coded `body`: walk from the binder to the nearest ancestor whose row has `Role::Scope` and take that ancestor's scope-bearing field — Rust `if_expression` / Java `if_statement` / Python `if_statement` `consequence` (`alternative` only where the language keeps the binding visible there, never for `if let`), Python/Go/JS `body`, Rust `match_arm` `value`, Rust `let_condition`/`let_chain` ⇒ the enclosing `if_expression.consequence` or `while_expression.body`. The row's `visibility` decides whether the introduction line is inside the span (`Header` ⇒ header-line uses inside the guarded consequence count, e.g. Java `if (o instanceof R(var x) && sink(x))`; `AfterIntroduction` ⇒ from the binder's end byte). `Empty` (a known scope with no lines, `if let Some(x) = opt {}`) demotes **nothing**; `Unknown` (no row, no scope-bearing ancestor field, census-only language) ⇒ the enclosing function from the binder line, fail-closed. `dominance_route_lines` (`:543-560`, `body`-only) is retired for this purpose and kept only by the lint. `lookup_uncertain` alone still takes the flat sets (Tasks 8–9 refine). The helper also consults `binding_facts.no_entry_binders` (Task 9's carrier) with the binder's body span as the doubt span.
- [ ] **Step 3: GREEN** — barrier module; `nav dfg-stats` counter = 2 on the barrier corpus; matrix fixture `java/dfg_binding_instanceof_pattern` (`doubt = "ownership_uncertain"`) → ok +1, 0/0; byte controls 0; label-delta lists exactly the Java instanceof edge; cap check. **Step 4: Commit** `enum(E0c-1): scope-bounded rule 1 — OwnershipUncertain inside the span, discharged at scope exit`.

---

### Task 8 (E0c-2): Rule 2a — classified non-`DefSite` masks by binding identity on both paths
**Behaviour question:** Can a mask ever change the label of a use outside its span, or fail to mask one inside it?
**Files:** `scope.rs` (`BindingFacts::new` `:127-210` pre-move: for each node whose selected row has `Role::Binding`, `declaration ∈ {Other, Pattern}`, ruling `Classified`, and whose introduced identifiers are not `DefSite`s, push a `Declaration` with the row's scope span; `javascript.rs` gains `catch_clause` (`Binding`+`Scope`, fields `["parameter"]`, `Other`, `WholeScope`)); `classify.rs` (on the `lookup_uncertain` path, before `classify_by_reaching_sets`: `binding_facts.mask_on_route(edge, use_byte)` — a classified mask declaration of the name visible at the use byte whose binding differs from the def's ⇒ `NameOnly(Killed{kill_line: binder line})`; the resolved path already yields `KilledAt` via `relation`); `barrier.rs`; `eval/fixtures/javascript/dfg_binding_catch_param/`.
- [ ] **Step 1: RED:** `js-catch-param-masks-declared` (`let x = source(); try { risky(); } catch (x) { sink(x); }` ⇒ `2:x→3:x` `Killed{3}`), `js-catch-param-masks-undeclared` (`x = source(); …` same ⇒ `Killed{3}`; RED on main — `FlatFallback` ⇒ `Exact`), `js-catch-after-try-preserved` (`sink(x)` after the `try` ⇒ `Exact`), `go-if-init-mask-not-route-kill` (`x := source()\n if x := f(); c { use(x) }\n sink(x)` ⇒ `2:x→3:x` `Exact` — every route passes the header but the header's `x` is a different binding). Run → the undeclared case FAILS `left: Exact`. Save `task8-red.log`.
- [ ] **Step 2: Implement** declaration synthesis + `mask_on_route` (binding identity within the span, never a route kill). **Step 3: GREEN** + fixture (`doubt = "killed"`, `kill_line = 3`); byte controls 0; `review_regressions.rs` unchanged; label-delta = the undeclared-catch edges only; cap check (`scope.rs` ≈ 480). **Step 4: Commit** `enum(E0c-2): rule 2a — classified non-DefSite masks by binding identity on the resolved and flat-fallback paths`.

---

### Task 9 (E0c-3): Rule 2b — reuse transfer on body-entry edges (ruling 3 replaced, W9)
**Behaviour question:** Is `Killed` reported only when no admissible route survives the reuse binder?

**Placement (the plan owns it; spec §6.4 rule 2b text unchanged).** The solver (`reaching.rs:258-296`) computes `out[n] = (in[n] − kill[n]) ∪ gen[n]` and propagates `out[n]` to **every** successor; `in[n]` excludes `n`'s own GEN. A loop header node (`cfg.rs:431-437`) has an exit edge to the statement after the loop, so a synthetic def generated at the header reaches the zero-iteration exit and kills the outer def there — the failure sol r1 W9 traced. Therefore the reuse transfer is applied **on the edges that enter the binder's body span, never at the binder's node**:
1. `transfer.rs::discover(parsed, func, defs) -> Vec<ReuseBinder { node_line, header_span, body_span, initializer_span, path, kind }>` runs **before** `BindingFacts::new` (it needs only `select_binding_row` and `defs`): nodes whose selected row is `Classified`, `declaration ∈ {Reuse, GoShort, JavaScriptVar, PythonAssignment}`, whose introduced identifier at that byte is **not** already a `DefSite` (Go `range_clause` `:=` *is* one via `ast.rs:4731/:9722`; `=` is not). **Bookkeeping (astra W2):** the synthetic `DefSite`s (`synthetic: true`, `start_byte` = the binder identifier's byte, `line` = the binder line, `alias_derived: false`) are appended to `defs` and `mapped_defs` **before** `BindingFacts::new(parsed, func, &defs, cfg)`, so `declaration_seed` gives each one its reused binding identity in `def_bindings` exactly like a real def (its ancestor row is a reuse kind ⇒ `reuses_binding_in_scope` ⇒ attaches to the visible declaration; an undeclared outer ⇒ `FlatFallback`, which `same_def_binding` treats as same-binding). `same_def_binding(new, old)` (`scope.rs:212-218`, indexing `def_bindings`) is then in bounds for every index the GEN/KILL loops (`reaching.rs:171-192`) visit. Regression `synthetic-defs-have-binding-entries`: after synthesis, `binding_facts.def_bindings.len() == defs.len()` and `same_def_binding(syn, i)` returns for every `i`. **Caps (astra S2):** synthesis runs after the `RD_MAX_DEFS` (`:117`) and `RD_MAX_LINES` (`:124`) checks; `transfer::discover` re-checks `defs.len() > RD_MAX_DEFS` (⇒ `DefinitionsCapExceeded { actual: post-synthesis count }`) and `transfer::apply` re-checks `node_count + synthetic_nodes > RD_MAX_LINES + 1` (⇒ `StatementLinesCapExceeded`), both recorded through `RdFileStats` as today; boundary regressions `caps-synthetic-defs-boundary` (`RD_MAX_DEFS − 1` real defs + 2 reuse binders ⇒ `Unavailable`; `RD_MAX_DEFS − 3` ⇒ `Available`) and `caps-synthetic-nodes-boundary` (analogous over `RD_MAX_LINES`).
2. For each binder with a body span: for every CFG edge `h → b` from the binder's node `h` into a node `b` whose line lies inside the body span, insert a synthetic node `s` (index ≥ `node_count`, **not** in `line_index`, so no def or use ever maps to it): replace `h → b` with `h → s → b` (both `incomplete` flags copied from the original edge); `gen[s] = {synthetic def}` and `kill[s]`/`flat_kill[s]` = the same-path defs exactly as `reaching.rs:174-192` computes them for a real def. Edges from `h` to lines outside the span (the loop exit, `else`, fall-through after a `with`) are untouched. A binder with no body span (statement-level reuse, e.g. Lua `assignment_statement` that is not a `DefSite`) gets `s` on every outgoing edge of `h`. **Same-line rule (sol r2 W1) and its mechanism (astra W3):** `s` is inserted before **every distinct-line body successor** of `h`. A use on the binder's header line is **not** covered by the existing collapse (`reaching.rs:371-372` needs `def_line == use_line` or a collapsed group, and a single synthetic def creates neither), so `classify_edge` gains an explicit check `binding_facts.header_use(edge, use_byte) -> Option<HeaderUse::{Body, Initializer}>` (carrier: `BindingFacts.header_spans: Vec<(Line, header_span, body_span, initializer_span)>`, filled from `transfer::discover`): a header-line use whose byte lies inside `body_span` is ambiguous between the outer def and the synthetic def on that line ⇒ `NameOnly(SameLine)`; a header-line use inside `initializer_span` (the `right`/`value` field: the range iterable, the `with` expression, the `for` iterable) reads **before** the binding ⇒ ordinary classification ⇒ the outer def reaches ⇒ `Exact`.
3. Each synthetic `DefSite` is mapped to its `s` node (`mapped_defs[syn] = Some(s)`; a binder with several body-entry edges gets one synthetic def per edge, all on the binder line); `matching_defs` (`classify.rs`) skips synthetic defs so no DFG edge is attributed to them; `lowest_reachable_kill` (`classify.rs`, was `reaching.rs:505-523`) iterates `line_index ∪ synthetic_nodes` where each synthetic node reports the **binder's line** as its kill line.
4. Both solvers see the same synthetic nodes (`solve_reaching_sets` is called twice with `kill` and `flat_kill`; `predecessors`/`successors` are shared).
**Consequence, traced:** Go `x := source()` (2); `for _, x = range items {` (3); `observe(x)` (4); `}`; `sink(x)` (6). Edges: 3→s→4, 4→3 (back), 3→6 (exit). `out[s] = {syn}`; `in[4] = {syn}` ⇒ outer not reaching ⇒ `lowest_reachable_kill` finds `s` (kill contains outer, route 2→3→s→4) ⇒ `Killed{3}`. `in[3] = out[2] ∪ out[4] = {outer} ∪ {syn}`; `out[3] = {outer, syn}` ⇒ `in[6] ∋ outer` ⇒ `definition_reaches_unflagged` ⇒ `Exact`. Conditional bypass `if c { for … }` ⇒ the `if`-false route carries `outer` ⇒ `Exact`.
**If the CFG lacks a header→body edge (S1 carrier):** `BindingFacts::new` gains a `cfg: &CfgView { line_index, successors }` argument (the CFG is built before `BindingFacts::new` is called, `reaching.rs:136-169`) and, during scope analysis, records `no_entry_binders: BTreeMap<String, BTreeSet<Line>>` — every classified reuse binder with a distinct-line body span whose node has **no** successor inside that span. Rule 1's route helper (Task 7) consults this map with the body span as the doubt span, so such a use is `OwnershipUncertain{binder line}` (fail-closed), and the census reports the count under a `no_body_entry` column. Nothing in `cfg.rs` changes. **Reachability, measured at afc78147:** the sequential fall-through (`cfg.rs:214-229`) links every consecutive statement pair whose predecessor is not a terminator, and a loop header is never a terminator, so no grammar occurrence with a distinct-line body statement reaches this fallback today; the regression is therefore harness-level — `no-entry-edge-fallback` constructs `BindingFacts` through a test-only `BindingFacts::with_cfg_view` whose view omits the header→body edge and asserts (a) the binder lands in `no_entry_binders` and (b) a use inside the span labels `OwnershipUncertain`. The case is kept so a future CFG change that drops the edge cannot silently produce `Exact`.

**Files:** create `transfer.rs`; modify `reaching.rs` (`reaching_definitions` `:110-256`: build the `CfgView`; `transfer::discover` + synthetic `DefSite`/`mapped_defs` extension and the cap re-check **before** `BindingFacts::new`; `transfer::apply` splices the synthetic nodes before the GEN/KILL loops; `DefSite` gains `synthetic: bool`), `scope.rs` (`BindingFacts::new` signature + `no_entry_binders` + `header_spans`/`header_use` + test-only `with_cfg_view`), `classify.rs` (`matching_defs`, `lowest_reachable_kill`), `go.rs` (`range_clause` rows: `variant: Some(OperatorIs{any_of:&[":="]})` ⇒ `GoShort`, `Header`; residual ⇒ `Reuse`, `AfterIntroduction`, body span = the `for_statement` body), `barrier.rs`, `eval/fixtures/go/dfg_binding_range_reuse/`.
- [ ] **Step 1: RED pair that distinguishes header placement from body-entry placement:** `go-range-reuse-inside-loop` (`2:x→4:x` ⇒ `Killed{3}`; RED on main: `Exact`), `go-range-reuse-zero-iteration` (`2:x→6:x` ⇒ `Exact`; GREEN on main and must stay — a header-placed transfer turns it RED), `conditional-bypass-preserved` (⇒ `Exact`), `go-range-short-var-no-synthesis` (`:=` ⇒ inside `Killed{3}` via the existing `DefSite`; `defs.len()` unchanged by `transfer::apply`), **`go-range-body-starts-on-header-line`** (`x := source()` (2); `for _, x = range items { observe(x)` (3); `sink(x)` (4); `}` ⇒ `2:x→4:x` `Killed{3}`; `2:x→3:x` `SameLine` via `HeaderUse::Body`), **`go-range-iterable-read-on-header`** (`x := source()` (2); `for _, y = range f(x) { observe(y)` (3) ⇒ `2:x→3:x` `Exact` via `HeaderUse::Initializer`), `synthetic-defs-have-binding-entries`, `caps-synthetic-defs-boundary`, `caps-synthetic-nodes-boundary`, **`no-entry-edge-fallback`** (S1, harness-level as above). Save `task9-red.log`.
- [ ] **Step 1b: Exact-equality cap cases (sol r4 W5, sol r5 W4).** `RD_MAX_DEFS` and `RD_MAX_LINES` live at `reaching.rs:26`; statement-line refusal today starts at `RD_MAX_LINES+1` (`reaching.rs:124`), while the planned post-synthesis TOTAL-NODE check counts statement lines plus synthetic nodes and accepts equality. The regressions therefore assert: defs — post-synthesis `defs.len() == RD_MAX_DEFS` accepted, `== RD_MAX_DEFS+1` refused (cap fallback, no over-claim); nodes — total nodes `== RD_MAX_LINES+1` ACCEPTED and `== RD_MAX_LINES+2` REFUSED, so an erroneous `>=` in the re-check fails a test. Each refusal case asserts the existing cap fallback label, never `Exact`.
- [ ] **Step 2: Implement** per the placement above. **Step 3: GREEN** + fixture (`Killed` inside, `exact` after); byte controls 0; `go/dfg_reaching_loop_carried*`, `go/dfg_reaching_go_short_var_if` unchanged; label-delta = the inside-loop edges only; cap check. **Step 4: Commit** `enum(E0c-3): rule 2b — reuse transfer on body-entry edges via synthetic nodes; Go range inside-loop killed, zero-iteration and bypass preserved`.

---

### Task 10: Slice 1 checkpoint (controller)
- [ ] Full unit + doc suites (awk, one aggregate); both byte controls 0 diff with populations recorded; `--matrix-only` = base ok + 3, 0/0; clippy delta 0; `cargo fmt --check`; cap test green; label-delta report over all fixtures (every moved label + row + the `worst()` re-rank effects); census JSON committed; `closed` still empty (no curation yet); rebase check (Task 0 Step 6) — on any advance rebind and re-run.
- [ ] `~/code/tools/records/binding-enumeration/slice-1-report.md`; stop for owner approval. **Slice 1 needs no quick verdict:** nominal bytes unchanged; the behaviour-visible changes are the three RED-on-main regressions plus lowered over-claims, each carried by a matrix fixture and a paired preservation case; E4's quick requirement binds the closeout (Task 28).

---

## Slice 2 — curation A (Tasks 11–19 in waves A–E; Task 19i integrates)

**Common shape.** (1) RED — cases in `tests/enumeration/<lang>.rs`, matrix fixtures only where a label moves vs main, `task<N>-red.log`; (2) GREEN — replace the task's provisional rows with curated rows (`fields` from `node-types.json`; `Classified` only when the case proves masking + scope exit, else `Uncertain` with reason + `revisit`); the task does **not** edit `mod.rs`/`closed`; (3) gates — module tests, `cargo test --lib cpg::reaching`, byte controls 0, matrix ok/0/0 with the wave's fixtures, cap check, label-delta in the report; (4) Opus review, cap 2; (5) commit `enum(<task>): …`. **Every row's acceptance names its partitions** (execution / bypass / initializer visibility / scope exit / combinations) and pairs each downgrade with a preservation case.

### Task 11 (E1a-1, wave A): Python reuse targets
**Question:** do function-level targets reuse (no block scopes) and survive zero-iteration routes?
**Rows:** `for_statement` (field `left`, `Reuse`, body span = loop body — the Task 9 transfer applies), `with_statement`/`with_item` `as_pattern` alias (`Reuse`, body span = the `with` body; **no exit edge bypasses the header**, so the binding persists after the statement), `except_clause`/`except_group_clause` alias (`Other`: masks, dead after the handler), `delete_statement` (kills the current binding), function-local `import_statement`/`import_from_statement`/`aliased_import` (`Other`), `global_statement`/`nonlocal_statement` (`Uncertain{reason:"non-local binding", revisit:"future:nonlocal"}`).
**Cases (W12-corrected):** `py-for-target-reuse-inside` (`x = source()` / `for x in it:` / `sink(x)` ⇒ `Killed{3}`), `py-for-target-zero-iteration` (`sink(x)` after the loop ⇒ `Exact` — preservation), `py-with-as-reuse-inside` (`with open(p) as x:` / `sink(x)` ⇒ `Killed{3}`), `py-with-as-persists-after` (`sink(x)` after the `with` block ⇒ **`Killed{3}`** — on normal completion the alias assignment persists; the outer value does not survive), `py-conditional-reuse-bypass` (`if c:` / `x = clean()` / `sink(x)` after ⇒ `Exact` — the genuinely bypassable preservation pair for statement-level reuse), `py-except-alias` (spec §7.1: `Killed{4}` inside, `Exact` after), `py-del-kills` (`del x` then `sink(x)` ⇒ `Killed{3}`), `py-import-alias-masks`, `py-global-uncertain` (`global x` then `sink(x)` ⇒ `OwnershipUncertain`), `py-nonlocal-in-nested-def-is-capture` (⇒ `CfgIncomplete`).

### Task 12 (E1a-2, wave A): Python match patterns and comprehension/lambda scopes
**Question:** are `case` pattern names field-addressable, else `Uncertain` within the arm's span only?
**Rows:** `match_statement`/`case_clause` (Scope); `case_pattern`, `class_pattern`, `as_pattern`, `keyword_pattern`, `pattern_list`/`tuple_pattern`/`list_pattern` targets (`Classified` where `node-types.json` names an identifier-bearing field, else `Uncertain{reason:"pattern identifiers not field-addressable", revisit:"future:python-patterns"}`); the four comprehension kinds keep `Visibility::Comprehension` (`scope_python.rs` stays code; the row records it).
**Cases:** `py-case-as-pattern-inside-arm` (`case [y] as x: sink(x)` ⇒ `Killed`/`OwnershipUncertain` per ruling), `py-case-outside-arm-preserved` (⇒ `Exact`), `py-comprehension-target-first-iterable` (port control: today's labels), `py-lambda-param-body-is-capture` (⇒ `CfgIncomplete`).

### Task 13 (E1b, wave B): Go range variants, case blocks, type-switch alias
**Question:** do the `:=`/`=` range variants select exactly one row each and follow 2a vs 2b respectively?
**Rows:** `range_clause` (Task 9's two rows — this task adds the `:=` masking case), `expression_case`/`default_case`/`type_case`/`communication_case` (Scope), `type_switch_statement` alias (`Other`, Header), `for_clause` init (`GoShort`, Header).
**Cases:** `go-range-short-var-masks` (`for _, x := range items { sink(x) }` ⇒ `Killed{3}`; after ⇒ `Exact`), `go-case-block-scope`, `go-type-switch-alias`, `go-for-clause-init` (header binding; preserved after the loop).

### Task 14 (E1c, wave C): Rust scopes and bindings
**Question:** does every block kind end an inner binding at its close?
**Rows:** `block`, `unsafe_block`, `const_block`, `try_block`, `match_arm`, `let_condition`/`let_chain` (Scope; Header for `if let`/`while let`), `closure_parameters` (`Other`, scope = closure body), `let_declaration`, `const_item`/`static_item`.
**Cases:** `rs-unsafe-block-shadow` (inside `Killed{3}`, after `Exact`), `rs-match-arm-scope`, `rs-if-let-header-scope`, `rs-try-block`, `rs-const-block`.

### Task 15 (E1d, wave C): Rust patterns
**Question:** does an uncertain pattern demote only inside its arm?
**Rows:** the eight pattern kinds (`Classified` only where a name field exists — verify `captured_pattern`; else `Uncertain{…, revisit:"future:rust-patterns"}`).
**Cases:** `rs-tuple-struct-pattern-inside-arm` (⇒ `OwnershipUncertain{3}`), `rust-if-let-uncertain-outside-scope` (Task 7's case re-listed), `rs-for-pattern-tuple` (inside per ruling, after `Exact`), `rs-captured-pattern`, `rs-closure-param-capture` (⇒ `CfgIncomplete`).

### Task 16 (E2a, wave D): JavaScript destructuring
**Question:** destructuring with defaults (`assignment_pattern` reading its own name): `Uncertain` or classifiable?
**Rows:** `object_pattern`, `array_pattern`, `assignment_pattern`, `rest_pattern`, `shorthand_property_identifier_pattern`; `pair_pattern`, `regex_pattern`, `class_heritage` (`NotBinding`).
**Cases:** `js-destructure-masks` (inside per ruling, after `Exact`), `js-destructure-default-reads-own-name` (`const {x = x} = o` ⇒ `OwnershipUncertain` unless proven), `js-rest-pattern`, `js-pair-pattern-not-binding` (invariance), `js-regex-not-binding` (invariance).

### Task 17 (E2b-1, wave D): TypeScript parameter and declaration kinds
**Question:** do TS parameter kinds seed like JS parameters? **Rows:** `required_parameter`, `optional_parameter`, `rest_parameter` (`Parameter`), `enum_assignment` (`NotBinding`), `abstract_class_declaration` (Scope). **Cases:** `ts-required-parameter-seeds` (param reaches ⇒ `Exact`; inner `let x` masks ⇒ `Killed`), `ts-optional-parameter`, `ts-rest-parameter`, `ts-enum-assignment-not-binding` (invariance).

### Task 18 (E2b-2, wave D): TSX and type-only kinds
**Question:** is every type-only kind transparent to the DFG? **Rows:** `type_alias_declaration`, `interface_declaration`, `ambient_declaration`, `enum_declaration`, `type_parameter(s)`, TSX `jsx_*` kinds the census flags — all `NotBinding`. **Cases:** one invariance case per row; `tsx-jsx-attribute-not-binding`.

### Task 19 (E3, wave E): Capture timing rows and the immediate counter (W10)
**Question:** does every boundary occurrence select exactly one capture row, stay `CfgIncomplete`, with `Immediate` counted only for invoked occurrences?
**Files:** `binding_table.rs` (`predicate_matches` implements `IsImmediatelyInvoked`: walk `parenthesized_expression` ancestors; the outermost wrapper — or the node itself — must be the `function` field of a `call_expression`); `capture_rows.rs` (`Immediate` variant rows for JS/TS `arrow_function`/`function_expression`, Python `lambda`, C++ `lambda_expression`); `capture.rs` (`CaptureFacts.ranges: Vec<(usize, usize, Timing)>` filled from `select_capture_row` at `:22`; `is_capture` returns `Option<Timing>` — the innermost enclosing boundary's timing); `reaching.rs` (`RdResult` gains `capture_immediate_edges: BTreeSet<(VarLocation, VarLocation)>`, filled beside `loop_carried_edges` `:223-245` when `classify_edge` reports a capture with `Timing::Immediate`; `RdFileStats` follows its existing identity-set pattern (`:73-108`): persisted `capture_immediate_edge_keys: BTreeSet<(RdFunctionKey, String)>` (edge id = `"from_line:path→to_line:path"`), derived `capture_immediate_edges: usize` via `refresh_counts`, `record_capture_immediate(function, start_line, edge_id)`, `merge` extends the set so PartialHit stays idempotent); `data_flow.rs:596` (instead of dropping, record each key of `result.capture_immediate_edges` into the file's `RdFileStats`); `build.rs:69-80` (`record_rd_stats` sums `capture_immediate_edges` into `dfg_label_capture_immediate`; the geometric `loop_carried` count at `:782` is untouched); `tests.rs::run` (`:118-135`) records the same keys into the `RdFileStats` it returns so `check_case`'s RD-stat accounting (Task 6, W3) observes the `+1`; `tests/enumeration/captures.rs`; `eval/fixtures/javascript/dfg_binding_iife/`.
**Cases:** `js-iife-parenthesized-invoked` (`(function(){ sink(x); })()` ⇒ `CfgIncomplete`, `expect_counter: Some(("dfg_label_capture_immediate", 1))`), `js-iife-arrow-invoked`, `js-iife-stored-parenthesized` (`const f = (function(){ sink(x); });` ⇒ `CfgIncomplete`, counter +0), `js-iife-mutation-w5` (`let x = source(); (() => { x = clean(); sink(x); })();` ⇒ `CfgIncomplete`), `py-lambda-called-in-place` (+1), `cpp-lambda-invoked-vs-stored` (+1 / +0), **`js-invoked-inside-stored-dedup`** (`const f = function(){ (() => sink(x))(); };` — the read's innermost boundary is the invoked arrow ⇒ counted once, +1; the same read is not also counted for the stored outer function) and **`js-stored-inside-invoked`** (`(function(){ const g = () => sink(x); })();` ⇒ innermost is stored ⇒ +0). Exclusivity test extended to `capture_rows`. Payload additions ride the Task 7 transition (history line amended, no second bump).
**Commit:** `enum(E3): capture timing rows with IsImmediatelyInvoked variants; dfg_label_capture_immediate carried through RdFileStats; every capture stays CfgIncomplete`.

### Task 19i: Slice 2 integration (controller)
- [ ] Merge waves A → B → C → D → E into `binding-enumeration` (`git merge --no-ff`); after each merge: module tests + `cargo test --lib cpg::reaching`. After E: append `"E1a-1","E1a-2","E1b","E1c","E1d","E2a","E2b-1","E2b-2","E3"` to `closed` in `tests/enumeration/mod.rs` (the only edit to that file; modules were registered by Task 6b); full unit + doc suites; byte controls 0; matrix ok/0/0 at the new count; clippy delta 0; cap test; label-delta report; provisional-row count per language (expected 0 for Python/Go/Rust/JS/TS/TSX except `future:*` rows); rebase check; `slice-2-report.md`; stop for owner approval.

---

## Slice 3 — curation B (Q1 default), Task 20a then waves F–I, Task 27i integrates

### Task 20a: Split `other.rs` before parallel work
- [ ] Pure move: `other.rs` → `java.rs`, `c.rs`, `cpp.rs`, `scripting.rs` (Lua, Bash, Terraform); `rows()` dispatch updated; the dynamic cap test (Task 1) covers the four new files with no edit (W6); all gates unchanged. Commit `enum(slice-3): split other.rs per language (pure move)`.

### Task 20 (E4a-1, wave F): Java scopes and local declarations
**Question:** does every Java block kind end its declarations at its close? **Rows:** `block`, `switch_block_statement_group`, `catch_clause`, `enhanced_for_statement`, `for_statement`, `try_with_resources_statement`, lambda body (Scope); `local_variable_declaration` (`Other`, fields `declarator`), `catch_formal_parameter` (`Other`), `enhanced_for_statement` variable (`Other`, Header). **Cases:** `java-block-shadow` (inside `Killed{3}`, after `Exact`), `java-catch-param-masks`, `java-enhanced-for-header`, `java-try-with-resources`, `java-switch-group-scope`.

### Task 21 (E4a-2, wave F): Java patterns and `instanceof`
**Question:** are pattern names field-addressable, else `Uncertain` within the guarded statement only? **Rows:** `record_pattern`, `type_pattern` (`Uncertain{…, revisit:"future:java-patterns"}` unless the census shows a `name` field), `instanceof_expression` guard span = the `if` consequence. **Cases:** `java-instanceof-pattern-uncertain` (Task 7's case re-listed), `java-instanceof-outside-guard-preserved` (⇒ `Exact`), `java-record-pattern-nested`.

### Task 22 (E4b, wave G): C
**Question:** do declarator chains introduce one binding per declarator at the declaration line? **Rows:** `compound_statement`, `for_statement` (Scope); `declaration`+`init_declarator` single (`Other`), multi-declarator chains `Uncertain{reason:"declarator chain", revisit:"future:c-declarators"}`. **Cases:** `c-block-shadow`, `c-for-init-scope`, `c-declarator-chain-uncertain` (`int a = 1, x = 2; sink(x)` ⇒ `OwnershipUncertain{2}`), `c/dfg_reaching_cfg_branch_arm_negative` unchanged.

### Task 23 (E4c, wave H): C++ scopes and bindings — as C plus `for_range_loop` (Scope + `Other` on `declarator`, Header), `catch_clause` (`Other`), lambda body (Scope). **Cases:** `cpp-range-for-masks` (inside `Killed{3}`, after `Exact`), `cpp-catch-param-masks`, `cpp-block-shadow`.

### Task 24 (E4d-1, wave H): C++ structured bindings — `structured_binding_declarator` `Uncertain{reason:"N bindings at one line; per-occurrence DefId is a future item", revisit:"future:cpp-structured"}`. **Cases:** `cpp-structured-binding-uncertain-inside` (⇒ `OwnershipUncertain{2}` within the block), `cpp-structured-binding-outside-preserved`.

### Task 25 (E4d-2, wave H): C++ `CaptureCopy` specifiers
**Question:** is the specifier read left to ordinary RD in the multiline form and pinned `cfg_incomplete` in the same-line form? **Rows:** `lambda_capture_specifier` (`Binding`, `CaptureCopy`, rule-2 exempt), `lambda_default_capture` (`NotBinding`). **Cases:** `cpp-capture-specifier-multiline` (specifier line ⇒ `Exact`; inner read ⇒ `CfgIncomplete`), `cpp-capture-specifier-same-line` (specifier-span edge ⇒ `CfgIncomplete` — pins the `(AccessPath, line)` collision, `capture.rs:9,:40-47`), `cpp-capture-not-a-mask` (`sink(x)` after the lambda ⇒ `Exact`).

### Task 26 (E4e-1, wave I): Lua — `block`, `for_numeric_clause`, `for_generic_clause`, `function_definition` (Scope); `variable_declaration` `local` (`Other`), `assignment_statement` (`Reuse` — statement-level, Task 9's no-body-span placement applies only where the lvalue is not already a `DefSite`). **Cases:** `lua-local-masks` (inside `Killed`, after `Exact`), `lua-assignment-reuse-bypass` (`x = source(); if c then x = clean() end; sink(x)` ⇒ `Exact`; inside the `if` after the assignment ⇒ `Killed`), `lua-for-clause-header`.

### Task 27 (E4e-2, wave I): Bash + HCL (census-only) — every candidate kind `Uncertain{reason:"census-only language", revisit:"future:scripting"}`, function span; HCL `variable_expr` `NotBinding`. **Cases:** the generated provisional cases; `unknown-scope-fail-closed` re-listed.

### Task 27i: Slice 3 integration (controller)
- [ ] Merge F → G → H → I; per-merge module tests; then append `"E4a-1","E4a-2","E4b","E4c","E4d-1","E4d-2","E4e-1","E4e-2"` to `closed`; full gates as Task 19i; provisional rows remain only with `future:*` revisits; `slice-3-report.md`; stop for owner approval.

---

### Task 28 (E5): Retire 1c, docs, closeout — **blocked on the Tier-A minimum comparative-closeout slice or owner Q3**
**Question:** truth pass — does every doc claim have a test or a measurement, and are both E4 gates satisfied?
- [ ] **Step 1: Finalize first (astra W9).** Steps 2–4 (heuristic demotion, join-by-kind test, docs) are implemented, gated (Step 5) and **committed**; the Step 6 rebase is done; the candidate SHA is recorded. Only then:
- [ ] **Step 1b: The comparative pair, for that exact SHA (W11).** E5's gate is a **recorded base/candidate comparative pair**, not a branch-only quick run: the Tier-A re-anchor spec §6 minimum slice **M0–M6** (`specs/2026-09-11-prism-tier-a-quick-reanchor-spec.md:170-177`; M6 = T4b′ "fresh base and candidate quick runs, two binaries, everything else held, comparative report with the §4.4 classes") produces it. Required record, saved under `~/code/tools/records/binding-enumeration/closeout/`: base SHA (the branch's rebase base) and candidate SHA (branch head); both binary digests; corpus identity (pinned tree digest + SHA alias), oracle identity (`rust-analyzer` version + `oracle_init` profile), runtime/toolchain identity, harness SHA; the comparison criteria (matrix `ok/gap/fail` equal or better; quick M2/M3 per-stratum precision/recall within the Tier-A spec's no-recall-loss rule; per-probe classes with zero `unexplained`); both runs' report JSONs and the comparative report. **Every contract field is validated, not merely present:** candidate SHA == the finalized head, base SHA == the rebase base, binary digests match the built artifacts, all non-SUT identities equal between legs, both legs with `meta.admitted == true` and `meta.baseline_invalid == false` (the producer contract exposes `baseline_invalid`, not `baseline_valid` — sol r5 W5) and comparative `criteria.both_baseline_valid == true`, every criterion `true`, verdict `NO_REGRESSION`. **Any subsequent change or rebase invalidates the pair — regenerate it for the new SHA.** **Alternative:** the owner's explicit E4 amendment for this item naming substitute evidence (Q3). If neither exists: STOP and report; the PR is not opened.
- [ ] **Step 2: Heuristic demoted** — `grammar_lint::derive_introducing_fields` is called only by the totality test and the runtime safety net (`select_binding_row` ⇒ `None` ⇒ `Uncertain{reason:"no table row"}`, counted); `introduction_is_classified` deleted. RED: a source-tree test asserting no caller of `grammar_lint` outside those two sites.
- [ ] **Step 3: Join-by-kind test** — for every `(language, kind)` in `capture_rows`, any binding-table row of the same kind is `Scope`-only or `CaptureCopy`; never `Reuse`/`Other`.
- [ ] **Step 4: Docs** — amendment 1d after the item 2 design's `:534` entry; B8 wording; the dfg-stats schema doc (item 2 §4.7) with both counters and the updated counter-sum identity (`ownership_uncertain ⊂ nameonly`; `capture_immediate ⊂ cfg_incomplete`); README truth; `VERIFICATION.md` rows in `~/code/tools`.
- [ ] **Step 5: Gates** (before Step 1b; re-run after any change) — full unit + doc suites (awk); both byte controls 0 with populations; `--matrix-only` ok/0/0 at the final count; clippy delta 0; exactly one `CACHE_VERSION` transition on the branch (`git log -p -- src/cpg_cache.rs | grep -c '^+const CACHE_VERSION'` = 1); cap test; control checksums recorded.
- [ ] **Step 6: Rebase** per Task 0 Step 6 (rebind control, hashes, baselines; re-run Step 5). **Step 7 (sol r4 W4): the E5 commit (docs, `VERIFICATION.md` rows, PR body at `records/binding-enumeration/pr-body.md` WITHOUT the pair section) is made BEFORE the candidate SHA is recorded; then Step 1b generates the Tier-A M6 comparative pair for exactly that SHA; the pair's paths are appended to the PR body in `~/code/tools` (not a prism commit); after this point no commit or rebase touches the branch — any review-driven change re-runs Steps 5–7 and regenerates the pair. Then the astra final review; the controller asks the owner for push + PR authorization.**

---

## Rulings this plan makes (spec ambiguities resolved)

1. **One PR, four checkpoints** — cached CPG labels need a cache bump per label-changing merge; the spec allows one transition (§8.3).
2. **Badness order (corrected in v2, sol r1 W8; scope corrected in v3, sol r2 W2):** `Killed < OwnershipUncertain < AliasUnstable < SameLine < CfgIncomplete < CallNameOnly`, reading spec §6.4's chain as reporting precedence; the adjacent-pair test pins it. The re-rank affects `worst()` at `data_flow.rs:212` and `reaching.rs:248` (merged duplicate keys) **and** `query.rs:719` (confidence folded along successive path edges), so it is classified as an **E4 behaviour change on the scoped/exact surfaces** — label-only, nominal bytes unchanged — gated by the matrix, with both an edge-level and a path-level delta report and two path-order regressions (Task 6a). `classify_edge`'s check order is unchanged.
3. **Rule 2b placement (replaced in v2, sol r1 W9; bookkeeping fixed in v4, astra W2):** the reuse transfer is applied on the body-entry edges through synthetic CFG nodes, never at the binder's node, and never where a genuine `DefSite` exists; synthetic defs are discovered and given binding identities before `BindingFacts::new`; `lowest_reachable_kill` reports the binder's line for a synthetic node. The v1 "synthetic def mapped to the header node, filtered from `matching_defs`" is withdrawn: filtering blocks attribution but not the header's GEN/KILL on the zero-iteration exit edge.
4. **Parameter rows and the byte-range test both run and must agree** (Task 6); deleting the byte-range test is a later follow-up.
5. **Provisional rows carry generated weak cases** so the totality and rows-have-regressions gates hold from E0b; their exemption from the ruling assertion is explicit, keyed on the literal reason `"not yet curated"`, and expires with `no_provisional_row_whose_revisit_task_closed` (v3, sol r2 W4); each curation task replaces them; `closed` is edited only by integration tasks; all wave test modules are registered by Task 6b (W7).
6. **Q1 default** applied to slice 3; **Q2** fail default assumed; **Q3** default (wait) is Task 28's precondition, satisfied by the Tier-A minimum slice M0–M6.
7. **Payload additions after Task 7** (Task 19's `RdFileStats` field) ride the single transition with an amended history line — item 2 precedent — never a second bump.
8. **Fixture count:** 22 capability names over 57 directories (recounted in Task 0).
9. **Task order 1 (splits) → 2–5 (ports) → 6 (rows) → 6a (scaffold) → 6b (gates) → 7–9:** the splits land first so the cap gate is satisfiable at the first touching task (astra W8); the doubt variant, its counters and the cache transition are scaffolded (6a) before any case can assert them (6b, 7); rule 1 (7) is the first producer of the variant.
10. **Synthetic defs are first-class `DefSite`s for bookkeeping** (binding identity, `mapped_defs`, alias flag, caps) and invisible only to edge attribution (`matching_defs`) — astra W2; header-line uses are classified by an explicit `HeaderUse` carrier, never by the equal-statement collapse — astra W3.
11. **Visibility spans come from the grammar's scope/guard fields** (`consequence`, `body`, `value`, …) with `Known/Empty/Unknown` distinguished — astra W1.

## Self-review
**Spec coverage:** §4.2 → 6, 11–27; §4.3 five gates → 6 (+ provisional closure); §5 → 1 (`IsImmediatelyInvoked` → 19); §6.1–6.2 → 11–27; §6.3 → 5, 19, 25; §6.4 rule 1 → 7, rule 2a → 8, rule 2b → 9 (placement per ruling 3), exemption → 25, regression pairs → 7–9 and every curation task; §7.1 → 6 + per-task acceptance; §7.2 → 7–9, 11–27; §7.3 → every gate (label-delta + 57 fixtures); §8 → 0, 10, 19i, 27i, 28; §8.3 → 7 (+ ruling 7); §8.4 → 7, 19; §8.5 → 5 + the cap test in every touching task; §9 → Tasks 1–28 one-to-one plus 19i/20a/27i; §10 → rulings 6; §11 risks 6, 8, 9 → Task 0 Step 6, Task 25, Task 28 Step 1.
**Placeholder scan:** none; every RED is assertion-level after a scaffold step; `IsImmediatelyInvoked` returns `false` until Task 19 (stated).
**Type consistency:** `rows`, `capture_rows`, `select_binding_row`, `select_capture_row`, `predicate_matches`, `grammar_digest`, `pinned_digest`, `census`, `Case{expect_counter}`, `check_case`, `all_cases`, `nodes_of_kind`, `DeclarationKind::{Reuse, Pattern, CaptureCopy}`, `FlowDoubt::OwnershipUncertain { construct_line }`, `DefSite.synthetic`, `RdResult.capture_immediate_edges`, `RdFileStats.{capture_immediate_edge_keys, capture_immediate_edges, record_capture_immediate}`, `BindingFacts::{new(.., cfg), with_cfg_view, no_entry_binders, header_spans, header_use}`, `HeaderUse`, `CfgView`, `transfer::{discover, apply}`, `grammar_lint::visibility_span`/`SpanKind`, counters `dfg_label_nameonly_ownership_uncertain` / `dfg_label_capture_immediate`, `transfer::apply` — identical across Tasks 1, 5–9, 19, 25, 28.

## Review record
**r1 — sol** (codex gpt-5.6-sol, xhigh, read-only, ctx `tools-enum-plan-sol-r1-20260911`, cwd detached `~/code/slicing-enum-review` @ afc78147): **FIX W=12 S=4.** Every finding re-verified at the source before folding (solver propagation `reaching.rs:258-296`; loop exit edge `cfg.rs:431-437`; `worst()` sites `data_flow.rs:212`, `reaching.rs:248`, `query.rs:719`, order test `flow_confidence.rs:137-152`; `find_parameters_node` in `src/ast.rs`; loop-carried counting at `build.rs:782` from geometry, `loop_carried_edges` dropped at `data_flow.rs:596`).

| # | Finding | Disposition |
|---|---|---|
| W1 | `--all-targets` excludes doctests. | FOLDED — Task 0 Step 4 runs `cargo test --doc` and aggregates both logs once. |
| W2 | Census loop omitted TS/TSX. | FOLDED — Task 0 Step 5 enumerates every `tree-sitter-*` crate via `cargo metadata` incl. both vendored TS schemas with distinct digests. |
| W3 | `merge-base --is-ancestor` misses linear advances. | FOLDED — Task 0 Step 6 compares the recorded full SHA with `origin/main` and rebinds review checkout, control, hashes, baselines. |
| W4 | Parameter invariant contradicted the Bash/Terraform exemption. | FOLDED — (v4+: Task 6, formerly Task 5) Step 1 asserts rows exactly where `find_parameters_node` is `Some`, with explicit Bash/Terraform assertions. |
| W5 | Task 6 RED was a compile error. | FOLDED — Task 6 Step 1 scaffolds; Step 2 captures assertion-level failures (digest `""`, empty census, missing case). |
| W6 | Gates vacuous: no `check_case`, zero-node exclusivity, no `expect_counter`. | FOLDED — Task 6 interfaces + gates: `rows_have_regressions` invokes `check_case`; node count > 0; `expect_counter`; selected-row and ruling assertions. |
| W7 | Task 7 RED could not compile. | FOLDED — Task 7 Step 1 scaffolds variant + exhaustive arms + counters + cache bump; Step 2 captures `Exact != OwnershipUncertain`. |
| W8 | Proposed ranks contradicted §6.4. | FOLDED — ruling 2 corrected to `Killed < OwnershipUncertain < AliasUnstable < SameLine < CfgIncomplete`; adjacent-pair test; merged-key effect listed in the label-delta. |
| W9 | Header-mapped synthetic def leaks to the zero-iteration exit. | FOLDED — ruling 3 replaced: transfer on body-entry edges via synthetic nodes (Task 9 placement text); no synthesis where a `DefSite` exists; the RED pair distinguishes placements; no `cfg.rs` change (fail-closed `Uncertain` if no body-entry edge). |
| W10 | `Immediate` could not reach `record_label`. | FOLDED — Task 19 carries timing `select_capture_row` → `CaptureFacts` → `is_capture: Option<Timing>` → `RdResult.capture_immediate_edges` → `RdFileStats` → `build.rs::record_rd_stats`; invoked-inside-stored / stored-inside-invoked dedup cases. |
| W11 | Branch-only quick run is not E5's comparative gate. | FOLDED — Task 28 Step 1 requires the recorded base/candidate pair (SHAs, digests, corpus/oracle/runtime identity, criteria, both reports) produced by the Tier-A minimum slice M0–M6, or the owner's E4 amendment. |
| W12 | `with … as x` persists; post-`with` `Exact` was wrong. | FOLDED — Task 11: `py-with-as-persists-after` ⇒ `Killed{3}`; preservation via `py-for-target-zero-iteration` and `py-conditional-reuse-bypass`. |
| S1 | Custody lacked literal commands and recounts. | FOLDED — Task 0 Steps 2–4. |
| S2 | `scope.rs` ≈ 480 before Task 8; no per-task cap check. | FOLDED — (v4+: Task 1, formerly Task 5) also moves the Python comprehension collectors (`scope_python.rs`), expected ≈ 400 → ≈ 480 after Task 8; the cap test runs in every touching task; next split pre-declared (`scope_seed.rs`). |
| S3 | Estimates not derived. | FOLDED — per-class arithmetic from `progress.md:23-30,:84-90` and LEDGER rows 186-190/194/204/209; seat time separated from overlapping wall time. |
| S4 | Parallelism and the shared `closed` registry undefined. | FOLDED — waves A–E / F–I with disjoint files; `closed` and `mod.rs` edited only by Tasks 19i/27i; `other.rs` split in Task 20a before slice-3 parallelism; integration order named. |

Nothing disputed in r1.

**r2 — sol** (codex gpt-5.6-sol, xhigh, read-only, ctx `tools-enum-plan-sol-r2-20260911`, cwd detached `~/code/slicing-enum-review` @ afc78147; **cap 2/2**): **FIX W=7 S=1.** Every finding re-verified at the source before folding (`query.rs:700-722` folds `path_confidence.worst(confidence)` along DataFlow edges; `build.rs:69-80` sums `RdFileStats` fields; `RdFileStats` identity-set pattern `reaching.rs:73-108`; `cfg.rs:214-229` sequential fall-through for the S1 reachability note).

| # | Finding | Disposition |
|---|---|---|
| W1 | Same-line handling suppressed synthesis for a whole body starting on the header line. | FOLDED — Task 9 placement: `s` before every distinct-line body successor; `SameLine` only for uses on the header line; regression `go-range-body-starts-on-header-line`. |
| W2 | `query.rs:719` folds confidence along path edges; an edge-only delta misses re-rank effects; the change is E4 behaviour. | FOLDED — Task 6a Step 3: `--path-delta` report, two path-order regressions, explicit E4 classification (scoped/exact surfaces, label-only, matrix-gated); ruling 2 scope corrected. |
| W3 | `expect_counter` read `record_label`, but `dfg_label_capture_immediate` lives in `RdFileStats`. | FOLDED — Task 6 `check_case` (5): RD-stat counters read the persisted accounting via the extended `run()`; Task 19 mirrors the identity-set pattern and records keys in the test helper. |
| W4 | Task 6 asserted a variant Task 7 had not created. | FOLDED — new Task 6a scaffolds variant, order, counters, cache transition before Task 6; provisional rows' exemption from the ruling assertion is explicit and expiring (ruling 5, 9). |
| W5 | Relative `custody.txt` paths from different cwds; review worktree at risk. | FOLDED — one literal `CUSTODY=` path in every Task 0 step and the Global Constraints. |
| W6 | Cap test hand-listed files; the Task 20a split escaped it. | FOLDED — (v4+: Task 1, formerly Task 5) cap test enumerates `src/cpg/reaching/**/*.rs` dynamically. |
| W7 | Waves could not register their test modules without editing the shared `mod.rs`. | FOLDED — Task 6 creates and registers all thirteen modules empty; waves fill their own file; 19i/27i edit only `closed`. |
| S1 | "No body-entry edge" had no runtime carrier or reachability proof. | FOLDED — Task 9: `BindingFacts.no_entry_binders` set during scope analysis from a `CfgView`; rule 1 consults it; harness-level regression `no-entry-edge-fallback`; reachability at afc78147 measured as unreachable through the grammar (sequential fall-through) and stated. |

Nothing disputed in r2. Series 12 → 7, closed-form ⇒ converging; cap of 2 reached; the disclosed extension was the astra final below.

**r3 — astra final** (codex gpt-6-astra, xhigh, read-only, ctx `tools-enum-plan-astra-r3-20260911`, cwd detached `afc78147`; the disclosed extension): **FIX W=9 S=2, closed-form — "preserve the plan and repair before dispatch".** Verdict key points: the body-entry transform is sound (body paths cross the kill; zero-iteration and bypass routes retain the outer def); both JS catch variants are identity masks with after-handler preservation; the capture exemption is correct; the re-rank's E4 classification is correct and deferral is unnecessary once RED and the census execution are repaired (matrix + corrected path census + real path regressions are interim evidence under spec §8.2, not E4 closeout); harness-only coverage of the missing-entry fallback is acceptable if production and harness share the constructor/classifier; the M6 report contract is checkable field by field. Every finding re-verified at the source before folding (pinned node-types: Rust `if_expression` and Java `if_statement` fields `alternative/condition/consequence`; `same_def_binding` indexes `def_bindings` at `scope.rs:213-214` while `BindingFacts::new` is built at `reaching.rs:169` before the GEN/KILL loops; `SameLine` only from `def_line == use_line` or a collapsed group, `reaching.rs:371-372`; `nav callers|callees` traverse call edges, and the fold at `query.rs:719` is reached through `taint_forward_labeled`/`taint_forward_cfg_labeled` from `taint.rs:11055`; no `target-dir` override in `.cargo/` or `Cargo.toml`; caps checked at `reaching.rs:117,:124` before synthesis).

| # | Finding | Disposition |
|---|---|---|
| W1 | Span helper searched only `body` and excluded the introduction line; known guards fell back to function scope. | FOLDED — Task 7: `visibility_span` from the grammar's scope/guard fields with `Known/Empty/Unknown`; header-line inclusion per row `visibility`; regressions `rust-if-let-uncertain-outside-scope` (kept, now a real control), `java-instanceof-same-line-guarded-use`, `rust-if-let-empty-span`; ruling 11. |
| W2 | Synthetic defs had no `def_bindings` entry ⇒ out-of-bounds in `same_def_binding`. | FOLDED — Task 9: discovery and synthetic `DefSite`/`mapped_defs` extension before `BindingFacts::new`; identity via `declaration_seed`; regression `synthetic-defs-have-binding-entries`; ruling 10. |
| W3 | Header-use `SameLine` expectation had no mechanism. | FOLDED — Task 9: explicit `HeaderUse::{Body, Initializer}` carrier and check; regressions for the body use (`SameLine`) and the iterable read (`Exact`). |
| W4 | Path census named `nav … --resolution`, which does not exist and traverses call edges. | FOLDED — Task 6a: adapter leg over `dfg_forward_reachable_labeled` + executable `--algorithm taint --taint-source` leg, non-empty assertions, errors retained. |
| W5 | Task 6a's RED followed the implementation. | FOLDED — scaffold keeps the existing relative ranks; RED on the ruled order; then re-rank. |
| W6 | Shared JS/TS/TSX rows failed the per-language case gate. | FOLDED — Task 6b: lookup by `(language, regression_id)`, one case instance per language. |
| W7 | Building in the review worktree violated custody. | FOLDED — Task 0: `~/code/slicing-enum-base` detached checkout with `CARGO_TARGET_DIR`; review worktree asserted clean. |
| W8 | Cap gate unsatisfiable before the split. | FOLDED — new Task 1 (splits + dynamic cap test); ports renumbered 2–5, rows 6; slice arithmetic updated; ruling 9. |
| W9 | Comparative pair went stale during closeout. | FOLDED — Task 28: finalize + commit first, regenerate the pair for that SHA, validate every field, invalidate on change/rebase. |
| S1 | Briefs must carry the semantic partitions. | FOLDED — Global Constraints (seats): nested mask / reuse within a mask / bypass / initializer preservation in every 6a–9 brief. |
| S2 | Synthetic growth after the cap checks. | FOLDED — Task 9: cap re-checks after synthesis + two boundary regressions. |

Nothing disputed in r3. **Next: one sol delta round verifies this fold (disclosed), then the plan goes to the owner for slice-1 approval; astra task-review is required for Tasks 6a, 6b, 7, 8, 9.**


**r4 — sol, delta on the astra fold** (codex gpt-5.6-sol, xhigh, read-only, ctx `tools-enum-plan-sol-r4-20260911`, cwd detached `~/code/slicing-enum-review` @ afc78147; the disclosed verification round): **FIX W=5 S=1, closed-form, converging (12 → 7 → 9 → 5).** Folded by the controller (the drafting lane was terminated by the session rate limit after writing v4).

| # | Finding | Disposition |
|---|---|---|
| W1 | Java header visibility had no executable rule (`WholeScope` provisional rows; `sink(x)` outside `consequence`). | FOLDED — Task 7 Step 1b: explicit binder-end-through-`consequence`(/`alternative`) span in `java.rs`, byte-range assertion. |
| W2 | Path-delta leg unexecutable: no fixture diffs, missing `--repo`, undefined "taint source", per-fixture emptiness allowed. | FOLDED — Task 6a Step 3 rewritten: adapter leg over all 57 fixtures with a per-fixture non-empty-path requirement; CLI leg generates diffs and passes `--repo`; subcommand verified at Task 0. |
| W3 | Task 0 still fetched/added worktrees through `~/code/slicing` and wrote the review checkout. | FOLDED — Task 0 Steps 1 and 3: independent clones from GitHub for implementation and control; the review worktree is only asserted (SHA + clean); `git worktree list` on the main checkout recorded unchanged. |
| W4 | Task 28 committed after generating the pair. | FOLDED — Step 7: E5 commit first, then the pair for that SHA; pair paths land in `~/code/tools`; no post-pair branch commits. |
| W5 | Cap regressions tested `limit−1`/`limit+1` only. | FOLDED — Task 9 Step 1b: exact-equality cases at `RD_MAX_DEFS` and `RD_MAX_LINES+1`. |
| S1 | Operative metadata still attributed the splits to Task 5. | FOLDED — custody table and review-record rows annotated with the v4 renumbering (Task 1 owns the splits). |

Nothing disputed in r4. **Next: one further disclosed sol delta on this fold (the owner's standing authorization to exceed caps while converging), then the plan goes to the owner for slice-1 approval; astra task-review remains required for Tasks 6a, 6b, 7, 8, 9.**


**r5 — sol, delta on the controller's r4 fold** (codex gpt-5.6-sol, xhigh, read-only, ctx `tools-enum-plan-sol-r5-20260912`, cwd detached `~/code/slicing-enum-review` @ afc78147): **FIX W=5 S=1 — every finding a defect in the r4 fold text.** Folded by the controller.

| # | Finding | Disposition |
|---|---|---|
| W1 | Step 6 fetched `origin` after the rename to `github` and re-ran Step 1 as a checkout. | FOLDED — Step 6 uses `github/main`, rebases explicitly, refreshes the base clone, re-runs only Step 1's assertions. |
| W2 | A continuous binder→`alternative` span includes the then-branch; case 1 passed under `WholeScope`. | FOLDED — Task 7 Step 1b: disjoint branch-aware spans per JLS §6.3.2.2; case 1 gains the after-`if` `Exact` assertion; negated case added. |
| W3 | The base binary cannot carry a new `--path-delta-dump` mode. | FOLDED — Task 6a Step 3: the census example is built unchanged in the base clone; two executables compared. |
| W4 | Equality cases used `RD_MAX_LINES+1` refused, but the total-node check accepts equality. | FOLDED — Task 9 Step 1b: `+1` accepted / `+2` refused for nodes; `== RD_MAX_DEFS` accepted / `+1` refused for defs. |
| W5 | M6 exposes `meta.baseline_invalid`, not `baseline_valid`. | FOLDED — Task 28 Step 1b checks `meta.admitted`, `meta.baseline_invalid == false`, `criteria.both_baseline_valid`. |
| S1 | Lines 40/82/85/444 still attributed the cap/splits to Task 5. | FOLDED — renamed to Task 1. |

Series 12 → 7 → 9 → 5 → 5 (flat; all closed-form; no design content in the last two rounds). **PARKED-FOR-OWNER: approve slice 1 with the required astra task-reviews (6a, 6b, 7, 8, 9) catching residuals RED-first, or authorize one more delta round.**

## Appendix — row inventory per language (from spec §6.1–6.3; the census refines counts)

| Language | Scope rows | Binding rows (declaration kind) | NotBinding rows | Capture rows | Task(s) |
|---|---|---|---|---|---|
| Python | function_definition, lambda, class_definition, 4 comprehensions, match/case clause | assignment/augmented_assignment/named_expression (PythonAssignment); parameters; for/with targets (Reuse); except alias, function-local imports (Other); del; global/nonlocal (Uncertain); match patterns (Classified or Uncertain) | — | function_definition, decorated_definition, lambda (Deferred); lambda IIFE (Immediate) | 1, 11, 12, 19 |
| JavaScript / TypeScript / TSX | statement_block, class_body, class_static_block, 3 function kinds, for_statement, for_in_statement (let/const header), catch_clause, switch_body | variable_declaration (JavaScriptVar); lexical_declaration/class_declaration (Other); formal_parameters + TS parameter kinds (Parameter); catch parameter (Other); destructuring patterns (Classified or Uncertain) | pair_pattern, regex_pattern, class_heritage, type_parameter(s), enum_assignment, type-only declarations, jsx_* | 5 function kinds + generator_function (Deferred); arrow/function_expression IIFE (Immediate) | 2, 16–19 |
| Go | block, if/for/switch/type_switch/select (Header), *_case | short_var_declaration (GoShort); var/const_declaration (Other); parameter_list; range_clause `:=` (GoShort, Header) / `=` (Reuse); type_switch alias; for_clause init | — | function/method_declaration, func_literal (Deferred) | 3, 9, 13 |
| Rust | block, unsafe_block, const_block, try_block, match_arm, let_condition/let_chain (Header), closure body | let_declaration/const_item/static_item (Other); parameters; closure_parameters (Other); 8 pattern kinds (Classified or Uncertain) | — | function_item, closure_expression, async_block, gen_block (Deferred) | 4, 14, 15 |
| Java | block, switch_block_statement_group, catch_clause, enhanced_for_statement, for_statement, try_with_resources_statement, lambda body, instanceof guard | local_variable_declaration (Other); formal/spread_parameter; catch_formal_parameter, enhanced_for variable, resources (Other); record/type_pattern (Uncertain by Q1) | — | method/constructor_declaration, lambda_expression (Deferred) | 5, 20, 21 |
| C | compound_statement, for_statement | declaration+init_declarator single (Other), chains (Uncertain by Q1); parameter_declaration | — | function_definition (Deferred) | 5, 22 |
| C++ | as C + for_range_loop, catch_clause, lambda body | as C + for_range_loop declarator, catch parameter (Other); structured_binding_declarator (Uncertain by Q1); lambda_capture_specifier (CaptureCopy) | lambda_default_capture | function_definition, template_declaration, lambda_expression (Deferred); invoked lambda (Immediate) | 5, 23–25 |
| Lua | block, for_numeric_clause, for_generic_clause, function_definition | variable_declaration `local` (Other); assignment_statement (Reuse); parameters | — | function_declaration, function_definition (Deferred) | 5, 26 |
| Bash / Terraform | — (census-only) | every candidate kind Uncertain "census-only language"; no parameter rows | variable_expr (HCL) | function_definition / block (Deferred) | 5, 27 |
