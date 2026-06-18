# Phase-2 Receiver-Typing — Execution Handoff (2026-06-17)

**Purpose:** resume-from-here map for the in-flight Phase-2a build (codex implement/review loop). Survives a
context compaction. Links the architecture-of-record, the slice/increment plan, every deferred item, the
execution infra, and the operational gotchas.

---

## 1. Status snapshot
- **Status: Phase-2a COMPLETE** on branch **`phase2-receiver-typing`** (off `rust-receiver-typing-design`, off
  `main`) — PR-1 ✅ + PR-2 ✅ + PR-3 ✅, all via the codex implement(high)/review(xhigh) loop. **Held for owner
  go on push/PR** (not pushed; no PRs).
- **Progress:** **PR-1 ✅ + PR-2 ✅ (read-inert) + PR-3 ✅ (the behavior change, Tier-A-measured PASS).** PR-2
  chain: T2.1 `48f15d9`, T2.2 `287f8d2`, T2.3 `cad4310`, T2.4 `e80644c`, PR-2-review-fix `3b94052`. PR-3 chain:
  T3.1 `059503c`, T3.2 `e7800e6`+`7d85dc9`, T3.3 `125f9a8`, T3.4 `ba140e9`, **final-review BLOCKER fold
  `842e46e`**. (`git log --oneline` is authoritative.)
- **HANG MITIGATION (apply to every dispatch):** wrap the bridge call in `timeout 1800`; in task specs tell
  implementer/reviewer to run ONLY `cargo test --lib` + `fmt`/`clippy -p prism --lib`/`build` and to NOT run
  the full `cargo test` or the `cli`/`frameworks` integration targets (they stall at macOS `_dyld_start` /
  run slow repo-wide `prism nav`, causing multi-hour hangs). The ORCHESTRATOR runs `--matrix-only` (seconds).
  If a run hangs: kill it, then salvage — verify the uncommitted tree (compiles + targeted tests + matrix)
  and commit on the implementer's behalf if sound (this happened on T2.3 → salvaged as `bd207e2`/`cad4310`).
- **NOT pushed / no PRs** — owner gates push/PR. Each task = one amended commit (clean history for later PR split).

## 2. Architecture-of-record (the design chain — read these first)
- **Spec (WHAT/WHY):** `docs/superpowers/specs/2026-06-17-prism-rust-receiver-typing-design.md` — **rev 7,
  CONVERGED** after 6 codex gpt-5.5 xhigh rounds. §1 goal/seam, §2 problem space + the precision/recall stance,
  §3 the typer + identity indices + the build-time materialization, §6 form→resolution table, §7 invariants,
  §9 phasing (2a/2b/3). §0.1–0.6 carry the full round-by-round review audit.
- **Plan (HOW):** `docs/superpowers/plans/2026-06-17-prism-rust-receiver-typing-phase2a.md` — **rev 4,
  PLAN-READY** after 4 codex plan-review rounds. 3 PRs / 14 tasks, each a TDD task with the failing test +
  exact `file:line`. The self-review at the bottom maps every spec § → task and every review fold.
- **Phase-1 predecessor (MERGED #102–#105):** `docs/superpowers/specs/2026-06-17-prism-rust-module-resolution-design.md`
  — the language-neutral scope graph (`src/name_resolution/`) this builds on.
- **Phase-IP Go precedent (shipped):** the Go receiver dispatch (`resolution.rs` `ReceiverClassifier` /
  `interface_impls` / arity #100) — the proven model this generalizes (spec §5).
- Memory: `project_prism_phase2_receiver_typing.md` (+ `project_prism_name_resolution_scopegraph`,
  `project_prism_phase_ip`).

## 3. The slices (3 PRs / 14 tasks)
**PR-1 — foundation, INERT (no edge change; gated by `--matrix-only` parity):**
- T1.1 ✅ `af5c5f6` MethodFacts (AST kind/has_self/recv_mode/arity_excl_self) — folded: `self: Box<Self>`.
- T1.2 ✅ `8e48fbd` TypeKey/ReceiverTypeKey/ReceiverOutcome + in-repo-first `resolve_type_path_to_type_scope`
  + `canonical_external` (new `src/resolution_identity.rs`) — folded: absolute `::std::` paths.
- T1.3 ✅ `8ea87fe` `methods_by_scope` (R1: defining-type scope) + `identity_complete` guard (G2) — folded:
  per-bucket completion (external trait buckets must be absent).
- T1.4 ✅ `4746d8d` `field_types` + `return_types` (scope-aware, Self/alias/cfg) — folded: cfg-gated alias
  collapse (now conditioned-or-omit, recall-safe). (SHA changed from `4df6ee0` after a history rewrite when a
  docs commit interleaved the amend.)
- T1.5 ✅ `50677d9` CACHE_VERSION 12→13 + pin test → 13; `--matrix-only` parity = 0 regressions (only the 2
  pre-existing python `expected_gap`s). **PR-1 COMPLETE + verified inert.**

**PR-2 — RustReceiverTyper + post-pass, READ-INERT (legacy `receiver_type` + inline classifier UNTOUCHED):**
- T2.1 `CallSite.receiver_outcome: Option<ReceiverOutcome{key,bare,recovery}>` (serde default; **cmp_key-
  EXCLUDED**); update ALL `CallSite` literals incl. `tests/name_resolution/*`, `tests/navigation/*`; bump
  CACHE_VERSION 13→14.
- T2.2 direct visible-binding lookup (F5: returns `Binding`/`Span`, not a `Candidate`) + `(FileId,def_byte)`
  local_facts.
- T2.3 the **new** build-time `RustReceiverTyper` (self/param/typed-let/constructor/field/return/wrapper;
  **path-preserving `type_syntax`**; new `ReceiverRecovery` variants FieldTyped/ReturnTyped/StdWrapperPeel +
  update the exhaustive match at `navigation/queries.rs:82-88`). Do NOT touch the inline `ExpandedClassifier`.
- T2.4 `rematerialize_rust_receiver_keys` post-pass: run the typer, resolve `type_syntax`→`ReceiverTypeKey`,
  set `receiver_outcome` **in place** on `calls` + `callers` (order-preserving; `BTreeSet` element = remove+
  reinsert, cmp_key-excluded so order holds); leave `receiver_type` unchanged; `--matrix-only` parity → PR-2.

**PR-3 — read path + the Tier-A gate (the ONE measured behavior change) — ✅ COMPLETE:**
- T3.1 ✅ `059503c` `combine_kind(cands, method_facts, recovery, arg_count, arg_spread)`: inherent-single→Exact;
  trait/wrapper-single→NameOnly; multi→TraitCha-demote; empty→drop; arity via `MethodFacts.arity_excl_self`.
- T3.2 ✅ `e7800e6`+`7d85dc9` R6 read branch on `site.receiver_outcome` (`InRepo` → methods_by_scope, empty+
  `identity_complete`→drop else bare via `oc.bare`; `External` → **isolated `extension_methods` index** (NOT
  bare methods — the `7d85dc9` wrong-edge fix); `Bare` → owner_lookup; `None` → residue). Go path
  (`receiver_type`) untouched.
- T3.3 ✅ `125f9a8` F6 incremental: rebuild identity indices + `rematerialize_rust_receiver_keys` on incremental
  (rebuild-together; don't merge stale).
- T3.4 ✅ `ba140e9` capability fixtures (`field_typed_recovery`, `return_typed_recovery`, `extension_trait_method`,
  `cross_module_no_collision`) — all `ok` (trait fixtures stay `ok` as NameOnly).
- **Final review BLOCKER fold ✅ `842e46e`:** the PR-3 cumulative review (codex xhigh) found R3/R3b pre-empting
  the materialized Rust `receiver_outcome` branch — a `x.m()` whose receiver-var name matched a type owner key
  resolved to the wrong owner (a wrong edge). Fixed: gate R3+R3b on `!rust_recv_materialized` so a materialized
  Rust receiver outcome drives R6 first (§3.3); regression test `rust_receiver_outcome_wins_over_owner_key_collision`
  (fails-before/passes-after). Re-review (codex xhigh): **APPROVE, no findings** (traced that `Foo::m()`
  path-qualified calls carry no receiver_outcome, so R1 type-qualified resolution is untouched).
- T3.5 ✅ **Tier-A 2a gate — PASS** (full writeup `/tmp/pr3-measurement-summary.md`): `--matrix-only` EXIT 0,
  **19/19 rust fixtures `ok`, 0 regressions**; `--quick` M2 (dogfood) **P=1.0, fp=0 across all strata,
  byte-identical pre→post the fix** — the PR-3 behavior change introduced zero FPs, recall preserved; the
  precedence fix moved 0 dogfood edges (no var==type collisions in prism's Rust → its value is
  correctness-completeness, proven by the regression test + matrix). `--quick` exits 2 only on the U-method
  oracle/adjudication coverage floor-fail (2 probes pending re-adjudication, `stale_adjudications` 3,
  `sut_error_rate` 0.0 — NOT a prism regression, identical pre/post; the 3 stale verdicts are flip-candidates
  for the PR description, adjudicated via the dual-adjudicator protocol, NOT self-adjudicated).

## 4. Deferred items (DO NOT LOSE)
**Inside Phase 2a (tracked tasks #42/#43):**
- **#42 recv_mode for typed-self** — `MethodFacts.recv_mode` is text-derived (`contains('&')&&contains("mut")`),
  so `self: Pin<&mut Self>` mis-maps to `SelfRefMut` instead of `SelfBy` (scans the inner type). **UNUSED in
  2a** (recv_mode feeds Phase-3 applicability only; has_self/arity/kind are correct). Fix before Phase-3
  consumes recv_mode: typed-self (parameter-with-pattern-`self`) → `SelfBy` unconditionally; restrict the
  `&`/`&mut` text check to `self_parameter` nodes. `src/call_graph.rs` ~:1427.
- **#43 split receiver-index builder out of `call_graph.rs`** — file is ~2591 lines (over the 600 guideline).
  Extract the methods_by_scope/field_types/return_types extraction to a focused module. Non-blocking (PR-2/3
  read the public fields, not the extraction). Do at convenience.

**Phase 2b (gated; separate plan):**
- Residue removal/tightening for **unrecovered** receivers — Tier-A-gated (the `eval/fixtures/rust/
  r6_single_owner_demote/` fixture pins the recall-risk; `let x = mystery(); x.frobnicate()` is a correct
  demoted edge field/return typing can't recover).
- `nav call-stats` telemetry: `ReceiverRecovery` + drop histogram.

**Phase 3 (precision refinements; spec §9):**
- Raise trait/wrapper-single **NameOnly → Exact** via trait-in-scope (resolve the trait in NS_TYPE from the
  call scope — the scope graph supports it) + applicability (`ReceiverPlace` + recv_mode — needs #42).
- Trait-object/generic-bound dispatch over `rust_provider.satisfaction` (the Rust `interface_impls` analog).
- Wrapper/`Deref`-aware dispatch (the `Arc::clone` precision fix; 2a preserves today's blind peel).
- Method-chain receivers (`a.b().c()`).
- Binding-types-on-the-scope-graph: fold `ty` onto `Binding`; harden `BindingRef` to a stable per-scope ordinal.
- **Full cfg-conditioned alias splitting** (beyond the recall-safe omit/condition floor landed in T1.4).
- Go field/return-typed receiver gaps; cross-package concrete-asserted keys.
- Python inheritance/MRO + TS receiver typers.
- `owner_lookup:486` cross-language overload-arity generalization (from Phase-IP).

## 5. Execution infrastructure
- Branch **`phase2-receiver-typing`**. Implementer commits per task (amend on fix). Owner gates push/PR.
- **Implementer:** `cd ~/code/a2a-bridge && ./target/release/a2a-bridge run-workflow rust-impl --input
  /tmp/recv-task-N.md --session-cwd /Users/wesleyjinks/code/slicing --config
  examples/a2a-bridge.rust-impl-codex.toml --out /tmp/recv-impl-N.md < /dev/null` (codex gpt-5.5 **high**).
- **Reviewer:** `run-workflow rust-code-review-recv --input /tmp/recv-review-N.md ... --config
  examples/a2a-bridge.rust-code-review-recv-codex.toml --out /tmp/recv-review-N.out.md < /dev/null` (gpt-5.5
  **xhigh**, read-only, points at the receiver-typing spec/plan).
- **Per-task loop:** write `/tmp/recv-task-N.md` (extract the plan task verbatim + spec refs + INERT/scope
  constraints + exact commit msg w/ trailer) → dispatch implementer (background) → **orchestrator diff-vs-base
  check** (scope + inertness, independent of the agent's self-test) → dispatch reviewer → fold findings
  (`/tmp/recv-task-N-fix.md` → implementer amends the task commit) → verify the delta → next task.
- **Task tracker:** `#28`–`#43` (TaskList). **DEFERRED:** #42, #43.

## 6. Operational gotchas (for resume)
- **macOS `_dyld_start` stall:** codex test runs (and direct `cargo test`) occasionally hang at loader startup
  before any test code — terminate + rerun the target once (transient, not a failure). A targeted `cargo test
  --test <name> < /dev/null` confirms a stalled target green (e.g. `frameworks` = 40/0).
- **Slow CLI dogfood tests:** the `cli` target's `nav_compat` tests run repo-wide `prism nav` (~3 min, not a
  hang) — allow time or `--test cli -- --test-threads=1`. (Also a latent `prism nav` perf signal.)
- **`tier-a --quick` exits 2** = `baseline_invalid` (rc 2; `cli.py:746`). On this branch the cause is the
  U-method stratum floor-fail (2 thin-stratum probes at `src/ast.rs:753`/`src/type_db.rs:482` pending
  re-adjudication; `stale_adjudications`>0 after a behavior change) — NOT drift and NOT a prism regression
  (`sut_error_rate 0.0`). `--allow-drift` covers the separate `corpus_sha_drift` cause. **Output goes to
  `eval/runs/<date>-prism.json` + `docs/eval/tier-a/`, NOT stdout** (empty stdout on the invalid path) — read
  the run JSON's `meta.invalid_reasons` / `m2` / `matrix`. `--matrix-only` (EXIT 0 = clean) is the
  authoritative inert+capability gate; the M2 numbers are the precision/recall payoff (with the pending-
  adjudication caveat).
- **Untracked run-artifacts** (`docs/eval/tier-a/2026-06-15-*`, `eval/snapshots/prism-*.json`) — leave
  untracked, NEVER commit (owner instruction). Each tier-a run adds a `prism-<sha>.json` snapshot.
- **Inertness contract:** PR-1/PR-2 must not change any resolved edge. `receiver_outcome` is cmp_key-excluded;
  legacy `receiver_type`/`receiver_recovery` + the inline `ExpandedClassifier` are untouched until PR-3.

## 7. Owner constraints
- **DO NOT push or open a PR until the owner asks.**
- codex gpt-5.5: implement **high**, review **xhigh** (keep xhigh for reviews).
- Commit trailer: `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`.
- Fold **blockers/majors** (verify against code first); minors are notes unless clustering.

## 8. Resume procedure
1. `cd /Users/wesleyjinks/code/slicing && git checkout phase2-receiver-typing && git log --oneline -8` (last
   task commit = where to continue).
2. `TaskList` → next non-completed task (#28–43); read this handoff §3 + the plan task.
3. If a fix was in flight (e.g. `b2ev2fjs4` T1.4 cfg-alias), read `/tmp/recv-impl-*.md` + `git log` to see if
   it landed; verify the delta; then continue.
4. Continue the §5 loop from the first non-done task. Hold for owner go before push/PR (§7).
