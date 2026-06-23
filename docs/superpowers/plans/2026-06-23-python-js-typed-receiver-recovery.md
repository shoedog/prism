# Python Typed-Receiver Recovery — Implementation Plan (rev 4)

> Strict TDD. Design-of-record: `…/specs/2026-06-23-python-js-typed-receiver-recovery.md` (rev 5, SHIP).
> Branch `slice2-typed-receivers` off merged main. codex-implement (orchestrator commits).
>
> **Rev 2 — codex plan-review fold (REWORK):** (BLOCKER) telemetry needs CallSite-level evidence →
> **DESCOPED** (Task 3 removed; buy measured via existing `kind_exact` deltas; skipped-split telemetry
> deferred). (BLOCKER) recovered `receiver_type` must **pre-empt R3/R3b** for Python/JS/TS (new T2) — else a
> receiver var name that collides with a class owner key binds via R3b before R6. (MAJOR) wildcard stored as
> a **sentinel in the imports map** (reuses existing threading, no new CallGraph field); singleton test
> asserts `receiver_type.is_none()` + no Exact-TypedParam (NameOnly residue allowed); Tier-A `--matrix-only`
> added per-AGENTS; TS param kinds named.

**Goal:** as spec — guarded Python typed-receiver recovery → Exact `TypedParam`/`ConstructorLocal` for
non-imported, non-wildcard-file local classes; sound; no external-drop spike; Rust/Go byte-identical.

> **Rev 3 — plan re-review-2 fold (REWORK, ordering only; design verdicts 1-4,6 TRUE):** (BLOCKER) Task 1
> must NOT commit recovery before Task 2's R3/R3b pre-emption — that intermediate is unsound (a receiver-var
> name colliding with a class owner false-Exacts via R3b). **Fix: land T1+T2 as ONE commit** (recovery +
> guard + R3b-pre-emption + R6-fallthrough together); the R3b-collision failing test is in the **first** red
> set so no recovery-enabled commit can pass without pre-emption. (MINOR) spec rev-4 §3.4/§7 still ask for
> the `skipped_*` telemetry split — **owner-deferred** (note in the PR checklist), buy measured via
> `kind_exact` deltas.
>
> **Rev 4 — fix re-review blocker fold:** narrow this slice to **Python-only**. JS/TS recovery is deferred
> because the binding walk descends into nested lexical blocks and can materialize a receiver that is not
> visible at the call site, suppressing legitimate R3 import-qualified resolution. JS buy is ≈0 on the
> Express/CommonJS corpus, so remove JS/TS recovery and add lexical-block non-regression tests that preserve
> R3.

**TDD order:** **T1+T2 = ONE commit** (Python gates + guarded recovery + wildcard sentinel + R3/R3b pre-emption +
R6 miss→fallthrough — recovery never lands without pre-emption) → T3 cache → T4 fixtures → T5 acceptance.
The two tasks below stay as separate TDD *steps* but share a single commit; the first failing-test set MUST
include the R3b-collision case.

---

## Task 1: gates + guarded recovery + wildcard sentinel
**Files:** `src/ast.rs` (`receiver_type_in_fn` `:424` gate + recovery cases; `walk_receiver_bindings`
`~:3982`; `constructor_type` `~:4103`; `extract_imports`/`import_from_statement` `~:638` wildcard sentinel);
`src/resolution.rs` (`recover_simple_ident` `:318` gate + post-recovery type-name guard).

- [ ] **Step 1 — failing tests** (real-source build + `resolve_call_site_full`), asserting on
  `site.receiver_type` AND the resolved kind so they can't pass pre-change:
  (a) Python local typed-param → `receiver_type==Some(Foo)` + Exact `TypedParam`; (b) imported `Foo` →
  `receiver_type.is_none()` (skipped); (c) wildcard file → `receiver_type.is_none()` (both class orders);
  (d) TS typed-param/annotation/`new Foo()` and JS `new Foo()` → `receiver_type.is_none()` and
  `receiver_materialized == false`; (e) JS/TS nested lexical-block binding repro preserves R3
  `ImportQualified`.
- [ ] **Step 2 — run-fail.**
- [ ] **Step 3 — implement:** open both gates for `Python` only; recovery in `receiver_type_in_fn`/
  `walk_receiver_bindings`: Python `typed_parameter`/`typed_default_parameter` (`type` field), `x = Foo()`
  (Python), `x: Foo` annotated-assignment. Remove JS/TS parameter `type_annotation`, `const x: Foo`, and
  `x = new Foo()` recovery. **Wildcard sentinel:** in
  `extract_imports`, on a `wildcard_import` child, insert a reserved key (e.g. `"*" -> "*"`) into the file's
  imports map (reuses `CallGraph.imports`/`ReceiverCtx.file_imports` — no new field/merge/cache plumbing).
  **Guard (in `recover_simple_ident`, on the PEELED type `T`, before storing `owner_key`):** return `None`
  if `T` is in `file_imports` OR `file_imports` contains the `"*"` sentinel. Language-gate to Python
  (Rust/Go untouched).
- [ ] **Step 4 — run-pass + build.** **Do NOT commit yet** — recovery must not land without T2's R3b
  pre-emption (else an unsound false-Exact window). Continue to T2; commit T1+T2 together.

---

## Task 2: R3/R3b pre-emption + R6 miss→fallthrough (Python)
**Files:** `src/resolution.rs` (the `Some(q)` qualifier arm R3/R3b `~:988-1042`; R6 recovered branch
`~:1110-1166`); tests.

- [ ] **Step 1 — failing tests:** (a) **R3b pre-emption:** Python `def f(x: Foo): x.m()` where the receiver
  var name OR an owner key collides — the recovered `Foo` type must win, NOT an R3b `owner_lookup(q,…)`
  binding; (b) recovered local type whose class **lacks** the method → NOT `dropped(ExternalReceiver)`;
  resolves identically to the un-annotated call (residue parity); (c) multi-owner hit → NameOnly;
  (d) Rust/Go recovered-miss → still `dropped(ExternalReceiver)` (byte-identical).
- [ ] **Step 2 — run-fail.**
- [ ] **Step 3 — implement:** mirror Rust's `rust_recv_materialized` pre-emption (`~:988`): for a
  `Python` site with `site.receiver_materialized`, **skip R3 (import-qualifier) and R3b
  (owner-key)** so the recovered type drives resolution (gate so Rust/Go behavior is unchanged). Then in the
  R6 recovered branch, on a Python `owner_lookup` **miss do NOT return** — fall through to residue
  (`~:1166`); keep `dropped(ExternalReceiver)` + the Go interface consult for Rust/Go only. Hits preserve
  confidence (owner_lookup demotes multi).
- [ ] **Step 4 — run-pass + `cargo test --lib`. Step 5 — commit T1+T2 TOGETHER** (one sound commit):
  `feat: guarded Python typed-receiver recovery + R3b pre-emption + miss-fallthrough`. The R3b-collision
  test must have been RED before this commit.

---

## Task 3: `CACHE_VERSION`
- [ ] Keep `CACHE_VERSION` at 25. It was already bumped for the additive `CallSite.receiver_materialized`
  field, which remains in this Python-only slice.

---

## Task 4: discriminating fixtures
**Files:** `tests/lang/python/`, `tests/lang/typescript/`, `tests/lang/javascript/` (+ mods + 3
`coverage_test.rs` arrays).
- [ ] Python typed-param/`x=Foo()`/annotation hit; shadow-bail; imported-skip; **wildcard-skip both orders**;
  **singleton external-vs-local:** `from ext import Foo; def f(x: Foo): x.m()` + in-repo `class Foo.m` →
  assert `receiver_type.is_none()` AND **no Exact `TypedParam`/`ConstructorLocal`** to the in-repo `Foo.m`
  (NameOnly residue is acceptable — the spec skips recovery, it does not poison residue); R3b-collision case;
  local-miss→residue parity.
- [ ] TS typed-param/`const x: Foo`/`new Foo()` and JS `new Foo()` **not** recovered
  (`receiver_type.is_none()`, `receiver_materialized == false`); JS/TS nested lexical-block binding repro
  preserves R3 `ImportQualified`. Rust/Go non-regression.
- [ ] Commit `test(slice2): typed-receiver discriminating fixtures`.

---

## Task 5: Acceptance (host-run; orchestrator)
- [ ] **Per AGENTS:** after T1/T2, `cargo build --release` then `cd eval && uv run tier-a --matrix-only
  --allow-stale-sut` (0-regr) before review.
- [ ] pydantic+fastapi main-vs-branch worktree call-stats: `kind_exact.typed_param`/`constructor_local`
  **rise**; `dropped_multi_owner`+`r6_single_owner` NameOnly **fall**; **`dropped_external_receiver`
  byte-FLAT**; canary `multi_target_exact_sites` byte-flat. **Report the realized buy — flag if negligible**
  (the import/wildcard guard shrinks it below the ~700 headline).
- [ ] **Rust (ripgrep)+Go (caddy) byte-identical** (owner accepts in lieu of `--quick`); Express/JS-TS flat.
  Suite + `--features mcp` + `fmt` green.

## Deferred (this slice)
JS/TS typed-receiver recovery (needs scope-aware lexical binding, unlike the discarded recursive binding
walk); `py_receiver_recovery { skipped_imported, skipped_wildcard }` telemetry (needs CallSite-level
skip-reason persistence — build/merge/cache plumbing); the buy is measured via `kind_exact` deltas instead.

## Self-review
- Spec coverage: T1 §3.1+§3.2.1+§3.3 (recovery+guard+wildcard sentinel); T2 §3.2.2 (fallthrough+confidence)
  **+ the R3b pre-emption the plan-review surfaced** (spec intent: recovered type drives resolution);
  T3 cache; T4 §7 fixtures (singleton asserts receiver_type.is_none); T5 §7 acceptance + Tier-A.
- Soundness-critical: the §3.3 guard (T1) + R3b pre-emption gated to Python (T2) + ExternalReceiver
  drop kept Rust/Go-only (T2).
