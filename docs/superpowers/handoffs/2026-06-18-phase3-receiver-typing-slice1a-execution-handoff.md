# Phase-3 Slice-1a — Execution Handoff (2026-06-18)

**Purpose:** resume-from-here map for executing the Phase-3 **Slice-1a** plan (in-repo method-chain receiver
typing, #5). Survives compaction. Links the architecture-of-record, the 7 tasks, the execution infra, gotchas,
deferred items, and owner constraints.

---

## 1. Status snapshot
- **Slice-1a plan is READY TO EXECUTE.** Spec converged (5 codex-xhigh rounds → rev 5 PLAN-READY); plan
  converged (3 plan-review rounds → rev 4 READY). **NEXT = execute PR-1a** via the codex implement(high)/
  review(xhigh) loop (subagent-driven-development).
- **Branch `phase3-receiver-typing`** (off `main`, **NOT pushed**). Key commits: spec rev5 (`f02e3a4` area),
  stratified call-stats telemetry (`49541a1`), plan rev4 (`3df88f8`). `git log --oneline main..HEAD` is
  authoritative.
- **Scope: IN-REPO RECALL ONLY.** Type the receiver of in-repo method chains (`a.b().c()`, `a.b(1).c(2).d()`,
  `let x = a.b(); x.c()`) → Exact (recall). Every external / Bare / StdWrapperPeel / non-unique / external-return
  receiver **fails closed to `None`** (today's `r6_single_owner` residue, unchanged). No external materialization,
  no drops. The external-drop *precision* story is a deferred follow-on (§7 of the spec).

## 2. Architecture-of-record (read first)
- **Plan (HOW):** `docs/superpowers/plans/2026-06-18-prism-rust-receiver-typing-phase3-slice1a.md` (rev 4, 1332
  lines, full code per task). THE execution contract.
- **Spec (WHAT/WHY):** `docs/superpowers/specs/2026-06-18-prism-rust-receiver-typing-phase3-design.md` (rev 5).
  §0 carries the 5-round audit trail; §2 the measured candidate analysis; §4 the design; §7 the follow-ons.
- **Parent:** Phase-2a (SHIPPED `main` #106/#107/#108 + minors #42/#43); its handoff
  `2026-06-17-phase2-receiver-typing-execution-handoff.md`. Memory: `project_prism_phase3_receiver_typing.md`
  (+ `project_prism_phase2_receiver_typing`, `feedback_workflow_preferences`).

## 3. The 7 PR-1a tasks (TDD; full code + tests in the plan)
- **T1** `method_call_parts` — AST decomposition of a receiver `call_expression` (`field_expression` function's
  `value`=receiver node + `field`=method; the call's `arguments` → arg count). Recurses on the receiver node →
  handles nested arg-bearing chains. (Replaces the rev-2 string splitter, which broke on `a.b(1).c()`.)
- **T2** `dispatch_method_single_exact` — mirrors `combine_kind` (`resolution.rs:494`): `StdWrapperPeel`→None;
  filter `has_self`+arity → single kept → require `MethodKind::Inherent` (borrow via `&fact.kind` — `MethodKind`
  is not Copy). Else None.
- **T3** `type_of_node` (AST chain recursion) + route the `call_expression` case through `method_chain_type`
  (recurse receiver → require `InRepo` → dispatch → in-repo `return_types` via `certain_index_type`,
  **rejecting External returns**); also route `type_from_local_fact`'s `InitExpr::Call` through it (the
  `let x = b.cfg(); x.run()` case). `type_of_expr` stays UNCHANGED (string leaves).
- **T4** fail-closed negatives (external / external-return-mid-chain / trait / StdWrapperPeel / depth-cap / cycle
  → `None`, residue unchanged).
- **T5** parallelize the serial `rematerialize_rust_receiver_keys` (`call_graph.rs:1123`): `let cg: &CallGraph =
  &*self;` → `cg.calls.par_iter()` map (build `RustReceiverTyper::new(cg)` in-closure) → collect → **sort by
  `(caller, CallSite::cmp_key)`** → serial apply. Deterministic; absorbs the recursion cost. Build-time:
  benchmark the cold CPG build (`time prism nav --no-cache call-stats --repo <tokio>`) branch vs main — no
  significant regression.
- **T6** capability fixtures (`eval/fixtures/rust/{chain_in_repo_exact, external_chain_unchanged,
  inrepo_then_external_unchanged}/`) — positives + multi-decoy negatives (empty residue so a wrong typed edge is
  observable) + `--matrix-only` 0-regression.
- **T7** confidence-aware gate: the **realized buy = `r6_single_owner_rust` reduction matched by `kind_exact`
  gains** (in-repo Exact recall), `dropped_external_receiver` **unchanged** (no external drops). Use the
  deterministic call-stats main→branch diff (the stratified telemetry is committed; tokio's Tier-A *oracle* run
  is invalid — 22% oracle error). Final codex-xhigh branch review → fold blockers/majors → re-review.

## 4. Execution infrastructure (the codex loop)
- **Implementer (code):** `cd ~/code/a2a-bridge && timeout 1800 ./target/release/a2a-bridge run-workflow
  rust-impl --input /tmp/recv-taskN.md --session-cwd /Users/wesleyjinks/code/slicing --config
  examples/a2a-bridge.rust-impl-codex.toml --out /tmp/recv-taskN.out.md < /dev/null` (codex gpt-5.5 **high**).
- **Reviewer (code):** `run-workflow rust-code-review-recv --config
  examples/a2a-bridge.rust-code-review-recv-codex.toml ...` (gpt-5.5 **xhigh**, read-only).
- **Plan draft/review (this session's workflow):** `rust-receiver-typing-phase3-plan-draft` /
  `...-plan-review` configs (codex drafts → I review → codex plan-reviews).
- **Per-task loop:** write `/tmp/recv-taskN.md` (extract the plan task verbatim + the in-repo-recall scope +
  recall-safety invariant + exact commit msg/trailer) → dispatch implementer (background) → **orchestrator
  diff-vs-base check** → dispatch reviewer → verify+fold findings (`/tmp/recv-taskN-fix.md` → amend) → next.
- **HANG/TIMEOUT mitigation:** wrap every dispatch in `timeout` (1800 impl; **2700 for big-plan reviews** — the
  1332-line plan review timed out at 1800); task specs run ONLY `cargo test --lib` + the specific tests + `fmt`/
  `clippy -p prism --lib`/`build -p prism`, **NOT** full `cargo test`/`cli`/`frameworks` (macOS `_dyld_start`
  stall). Orchestrator runs `--matrix-only` (seconds). If a run hangs: kill, salvage (verify uncommitted tree
  compiles+tests+matrix), commit on its behalf if sound.

## 5. Gotchas
- **AST, not strings:** `receiver_expr_text:365` normalizes only the OUTER call's args to `(...)`; arg-bearing
  intermediates break a string parser. The chain walk MUST be AST-node-based (T1).
- **`methods_by_scope` is a multi-candidate `Vec`** (inherent+trait dual-keyed); single-Exact = exactly one
  has_self candidate that is Inherent (NOT StdWrapperPeel). `return_types`/`field_types` keep only `InRepo`
  (so external returns can't be typed → fail closed).
- **Parallel post-pass determinism:** sort updates by `(caller, cmp_key)` before apply (`cmp_key` excludes
  `receiver_outcome`); `let cg = &*self` reborrow so the rayon map isn't seen as `&mut self`.
- **`tier-a --quick` exits 2** = `baseline_invalid` (oracle/adjudication coverage floor-fail, `sut_error_rate
  0.0`), NOT a regression; `--matrix-only` (EXIT 0) is the authoritative inert+capability gate. Read
  `eval/runs/<date>-prism.json` `meta`/`m2`/`matrix`, not stdout.
- **Untracked run artifacts** (`docs/eval/tier-a/2026-06-*`, `eval/snapshots/*`) — NEVER commit (owner).
- **Stratified telemetry** (committed `49541a1`): `nav call-stats` now emits `r6_single_owner_rust`,
  `kind_exact`/`kind_nameonly`, `nameonly_by_recovery_methodkind`, `wrapper_peel_clone_demotes` — the T7 gate
  reads these. call-stats flags: `nav --no-cache call-stats --repo <dir>` (no `--format`).

## 6. Deferred follow-ons (corrected numbers, review fixes folded — spec §7)
- **Slice-1b — #1 in-repo field/let identity widening:** widen `resolve_type_path_to_type_scope` coverage
  (re-export / `mod::Type` / more let forms), `None→typed` only (`Bare→identity` is a further Tier-A-gated
  follow-on). The `field_chain_exact` fixture is 1b, not 1a. Its own plan.
- **External-return / generic-output summaries:** the precision half (external chains/locals → drop-on-empty;
  generic re-entry `Option<Foo>.unwrap().m()`). Requires extending `return_types`/`field_types` to carry
  external/generic outputs.
- **#3 dyn-Trait / generic-bound dispatch** (`trait_cha`: 159 prism / **3,320 tokio** — next-biggest, over
  `rust_provider.satisfaction`). **#2** single-trait→Exact (~20/66, concrete-only + `trait_scope` identity not
  bare name). **#4** wrapper (Arc/Rc clone, wrapper-kind preserved, ~0). **#6** cfg-alias.

## 7. Owner constraints + the re-scope lessons
- **NOT pushed; no PRs** until the owner asks. On READY → execute the codex loop; if a review is FLAWED with
  blockers/majors → verify → fold verified → re-review (the standing instruction).
- codex implement **high** / review **xhigh**, model **gpt-5.5**. Commit trailer:
  `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`. PR bodies end with the Claude Code
  attribution.
- **Stacked-PR merge gotcha** (from Phase-2a, [[feedback_workflow_preferences]]): rebase/squash-merge rewrites
  SHAs → the next stacked PR goes CONFLICTING → rebuild the remaining stack on the new `main` tip via
  `commit-tree` + force-push.
- **The buy lesson:** don't claim buy from an aggregate metric — STRATIFY (confidence×kind×recovery) + SAMPLE
  first. The #2 buy was ~100× overclaimed; the call-stats also mixes languages (filter `.rs`). The dual-review
  loop caught a real soundness hole every round.

## 8. Resume procedure
1. `git checkout phase3-receiver-typing`; read the plan (§3 above) + spec §4.
2. Build the release SUT; run `--matrix-only --allow-stale-sut` to confirm a clean 19/19-rust baseline.
3. Execute T1→T7 via the codex implement(high)/review(xhigh) loop (§4), TDD per task, one commit per task.
4. T7 gate (the call-stats main→branch buy + matrix) + a final codex-xhigh branch review; fold; re-review.
5. Report the result; **hold for owner go on push/PR** (then assemble — note the stacked-PR merge gotcha).
