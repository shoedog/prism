# Python/JS Typed-Receiver Recovery — Implementation Plan

> Execute task-by-task, strict TDD. Design-of-record:
> `docs/superpowers/specs/2026-06-23-python-js-typed-receiver-recovery.md` (rev 4, SHIP — read §3). Branch
> `slice2-typed-receivers` off merged main. **codex-implement** (orchestrator commits — sandbox can't write
> `.git`).

**Goal:** resolve `x.method()` for Python/JS/TS recoverable receivers (typed params, constructor locals,
annotations) of **non-imported, non-wildcard-file** local classes → R6 `owner_lookup` → Exact
`TypedParam`/`ConstructorLocal`; sound (no external false-Exact), no `dropped_external_receiver` spike,
Rust/Go byte-identical.

**TDD ordering (each task's end-state sound):** T1 (gates + recovery **+ the import/wildcard guard
together** — never an unguarded false-Exact intermediate) → T2 (R6 miss→fallthrough) → T3 telemetry →
T4 cache → T5 tests → T6 acceptance.

---

## Task 1: open gates + guarded Python/JS/TS recovery
**Files:** `src/ast.rs` (`receiver_type_in_fn` `:424` gate + recovery cases; `walk_receiver_bindings`
`~:3955-4074`; `constructor_type` `~:4106`); `src/resolution.rs` (`recover_simple_ident` `:320` gate + the
post-recovery import/wildcard guard); `src/call_graph.rs` or `src/ast.rs` (wildcard sentinel in
`extract_imports`/`import_from_statement` `~:638`); tests in `src/resolution.rs`.

- [ ] **Step 1 — failing tests** (real-source `CallGraph::build` + `resolve_call_site_full`):
  (a) Python local typed-param `def f(x: Foo): x.m()` with same-module `class Foo: def m()` → **Exact
  `TypedParam`**; (b) imported `from ext import Foo; def f(x: Foo): x.m()` + in-repo `class Foo.m` → **NOT
  Exact** (skipped); (c) wildcard `from ext import *; def f(x: Foo): x.m()` + in-repo `class Foo.m` → **NOT
  Exact** (skipped), in both class-decl orders; (d) JS `const x = new Foo(); x.m()` local → Exact; bare JS
  `const x = Foo(); x.m()` → NOT recovered.
- [ ] **Step 2 — run-fail** (today: Rust/Go-gated → all unresolved/NameOnly).
- [ ] **Step 3 — implement:**
  - Open both gates for `Python|JavaScript|TypeScript|Tsx`.
  - Recovery in `receiver_type_in_fn`/`walk_receiver_bindings`: Python `typed_parameter`/
    `typed_default_parameter` (`type` field), `x = Foo()` constructor (Python only), `x: Foo` annotated
    assignment; TS parameter `type_annotation` (strip `:`), `const x: Foo`, `x = new Foo()`
    (`new_expression`). Reuse one-binding+shadow-bail.
  - **Wildcard sentinel:** in `extract_imports`, detect the `wildcard_import` child of
    `import_from_statement` → a per-file flag (e.g. an entry in/beside the imports map).
  - **Guard (in `recover_simple_ident` / classifier, post-recovery, peeled type `T`):** return `None`
    (skip) if `T` is in the file `imports` map OR the file has the wildcard flag. (Gate this to
    Python/JS/TS — Rust/Go unchanged.)
- [ ] **Step 4 — run-pass** + `cargo build`. **Step 5 — commit** `feat: guarded Python/JS typed-receiver recovery (gates + import/wildcard skip)`.

---

## Task 2: R6 miss → fallthrough (Python/JS), preserve confidence
**Files:** `src/resolution.rs` (R6 recovered branch `~:1110-1166`); tests.

- [ ] **Step 1 — failing tests:** (a) Python recovered local type whose class **lacks** the method
  (`def f(x: Foo): x.gone()`, `Foo` has no `gone`) → **NOT `dropped(ExternalReceiver)`**; resolves via R6
  residue exactly as the same call resolves WITHOUT a type annotation (parity); (b) multi-owner hit
  (two in-repo `class Foo` with `m`) → **NameOnly** (demoted), not Exact; (c) Rust/Go recovered-miss → still
  `dropped(ExternalReceiver)` (byte-identical).
- [ ] **Step 2 — run-fail** (today the recovered branch early-returns `dropped(ExternalReceiver)` on miss).
- [ ] **Step 3 — implement:** restructure the R6 recovered block so a `Python|JS|TS|Tsx` caller's
  `owner_lookup` **miss does NOT return** — fall through to the residue path (`~:1166`). Keep the
  `dropped(ExternalReceiver)` only for Rust/Go (+ the existing Go interface consult). Hits relabel
  preserving confidence (singleton Exact / multi NameOnly — `owner_lookup` already demotes).
- [ ] **Step 4 — run-pass** + `cargo test --lib`. **Step 5 — commit** `feat(resolution): Python/JS recovered-receiver miss falls through to residue (not external-drop)`.

---

## Task 3: telemetry
**Files:** `src/navigation/queries.rs` (+ stats struct); tests.
- [ ] Add `py_js_receiver_recovery { hit, miss_fallthrough, skipped_imported, skipped_wildcard }` counters,
  incremented at the recovery/guard/R6 sites. Test: a fixture exercising each bucket reports the right
  counts. Commit `feat(nav): py_js_receiver_recovery telemetry`.

---

## Task 4: `CACHE_VERSION` bump
- [ ] `src/cpg_cache.rs` 23→24 (+ assertion test). TDD assertion-first. Commit `chore(cache): CACHE_VERSION 23->24`.

---

## Task 5: discriminating fixtures
**Files:** `tests/lang/python/`, `tests/lang/typescript/`, `tests/lang/javascript/` (+ mods + 3
`coverage_test.rs` arrays).
- [ ] Python: typed-param hit, constructor-local `x=Foo()` hit, annotation hit, shadow-bail, imported-skip,
  **wildcard-skip (both orders)**, **singleton external-vs-local false-Exact → NOT bound**, local-miss→residue.
- [ ] TS: typed-param hit, `const x: Foo`, `new Foo()` hit, bare `Foo()` NOT recovered.
- [ ] Rust/Go non-regression fixture. Commit `test(slice2): typed-receiver discriminating fixtures`.

---

## Task 6: Acceptance (host-run; orchestrator)
- [ ] `cargo build --release`; pydantic+fastapi main-vs-branch worktree call-stats: `kind_exact.typed_param`/
  `constructor_local` **rise**; `dropped_multi_owner`+`r6_single_owner` NameOnly **fall**;
  **`dropped_external_receiver` byte-FLAT**; canary `multi_target_exact_sites` byte-flat. **Report the
  realized buy** (likely modest after the import/wildcard guard — flag if negligible).
- [ ] **Rust (ripgrep)+Go (caddy) byte-identical**; Express/JS flat. Tier-A `--matrix-only` 0-regr; suite +
  `--features mcp` + `fmt` green. Report `py_js_receiver_recovery` split.

---

## Self-review
- Spec coverage: T1 §3.1+§3.2.1+§3.3 (recovery+guard together = sound intermediate); T2 §3.2.2 (fallthrough+
  confidence); T3 §3.4; T4 cache; T5 §7 fixtures incl. singleton+wildcard-both-orders; T6 §7 acceptance.
- Ordering: guard lands WITH recovery (T1) so no unguarded-false-Exact commit; fallthrough (T2) before
  measuring external-flat.
- Soundness-critical: the §3.3 guard (T1 step 3) + the Rust/Go gate on the ExternalReceiver drop (T2).
