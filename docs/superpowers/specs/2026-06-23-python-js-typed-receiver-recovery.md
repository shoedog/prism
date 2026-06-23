# Python/JS Typed-Receiver Recovery — Design (2026-06-23)

> Slice 2 of the Python/JS resolution-maturity loop (after 1a self-same-class #131 + decorated #132).
> Basis: the codex xhigh architect analysis (memo `/tmp/slice2-architect-out.md`). Branch
> `slice2-typed-receivers` off merged main.

## 1. Problem
`x.method()` where the receiver `x`'s static type is **syntactically recoverable** — a typed param
(`def f(x: Foo)`, TS `f(x: Foo)`), a constructor local (`x = Foo()`, `x = new Foo()`), or an explicit
annotation — is currently **unresolved or NameOnly-demoted** for Python/JS/TS. The existing P6-lite
receiver recovery is **`Rust|Go`-gated** at two sites: `receiver_type_in_fn` (`src/ast.rs:403`) and
`recover_simple_ident` (`src/resolution.rs:320`). Rust/Go feed the recovered type into R6
(`owner_lookup(recv_ty, name)`, `src/resolution.rs:1110`) and relabel to `TypedParam`/`ConstructorLocal`;
Python/JS get nothing.

**Measured buy (architect, source-AST sizing of owner-lookup hits):** ~**171 FastAPI** + ~**542 pydantic**
recoverable `x.method()` sites that hit an in-repo owner (currently in `dropped_multi_owner` +
`r6_single_owner` NameOnly). **Express ≈ 0** (uses CommonJS `Router`/prototype, not in-repo ES classes) →
JS gets a guard fixture, no JS-specific buy this slice.

## 2. Goal
Recover the receiver's syntactic static type for Python/JS/TS (typed params, constructor locals, explicit
annotations) and route it through the existing R6 `owner_lookup`, resolving `x.method()` to the in-repo
owner class's method (`TypedParam`/`ConstructorLocal`) — **without** a `dropped_external_receiver` spike and
**byte-identical for Rust/Go**.

## 3. Mechanism — Option B "hit-or-fallthrough" (architect-recommended)
- **Open the gates** for `Python | JavaScript | TypeScript | Tsx` in `receiver_type_in_fn` (`ast.rs:403`)
  and `recover_simple_ident` (`resolution.rs:320`), reusing the existing `ReceiverClassifier` /
  `RecoveredReceiver { static_type, recovery }` plumbing (`resolution_receiver.rs:31-108`).
- **Add Python/JS/TS recovery cases** to the AST scan (`receiver_type_in_fn` + `walk_receiver_bindings`):
  typed params (`name: Type`), constructor locals (`x = Foo()` / `x = new Foo()`), explicit local
  annotations (`x: Foo`). Reuse the existing "one binding before the call + shadow-bail" semantics.
  `call_function_qualifier` already yields the receiver for Python `attribute` / JS `member_expression`
  (`languages/mod.rs:723`).
- **R6 routing with language-gated miss behavior (the key soundness point):** when `site.receiver_type` is
  set, `owner_lookup(recv_ty, name)`:
  - **hit** → relabel `TypedParam`/`ConstructorLocal` (as today).
  - **miss** → for `Python|JS|TS|Tsx`, **fall through to the R6 residue** (the existing unqualified/method
    path), **NOT** `dropped(ExternalReceiver)`. Rust/Go keep drop-on-miss (byte-identical). Rationale:
    FastAPI has ~1,416 syntactic recoveries with **no** in-repo owner (external types) — dropping them
    would spike `dropped_external_receiver` and lose the R6-residue resolution they get today.
- **Telemetry:** `py_js_receiver_recovery { hit, miss_fallthrough }` in call-stats (final-kind buckets
  don't show the recovered-but-missed set).

## 4. Soundness
- **Multi-owner:** `owner_lookup` already **demotes** a >1 same-name-owner pool to NameOnly
  (`resolution.rs:773`) — no multi-owner wrong-Exact.
- **Singleton external collision (the residual risk):** a recovered type `Foo` that is actually an
  **external** type but collides with a single **in-repo** class `Foo` → false Exact. Mitigation
  (first-merge, conservative): **skip recovery when the type name is an import-bound name** (present in the
  file's `imports` map) — an imported type is likely external/cross-module; resolving it properly is slice
  3/4. Recover only locally-defined-class types + constructor locals of local classes. This trades some
  recall for soundness; quantify the skipped set via telemetry.
- **Rust/Go unchanged** by the language gate + drop-on-miss preserved.
- The merged `method_class_span` identity is **self-only**; slice 2 uses bare `owner_lookup` (with the
  demote-on-multi + skip-imported guards). Span-keyed typed-receiver identity is a later refinement.

## 5. Scope
**In:** open the 2 gates for Python/JS/TS; Python/JS/TS recovery for typed params + constructor locals +
explicit annotations of **local** classes; R6 miss→fallthrough for Python/JS; skip import-bound type names;
telemetry; tests. **Out (defer):** cross-module/imported type resolution (slice 3/4), Python attribute
field types (`self.x: Foo`), TS structural/interface typing, CommonJS/prototype owner extraction (Express),
factory-return typing, chained-receiver typing, span-keyed typed-receiver identity.

## 6. Files
- `src/ast.rs` — `receiver_type_in_fn` (`:403` gate + Python/JS/TS param/local/constructor cases);
  `walk_receiver_bindings` (`~:3955-4074`, add Python/JS arms).
- `src/resolution.rs` — `recover_simple_ident` (`:320` gate + skip-import guard); R6 step 1 (`:1110`)
  miss→fallthrough for Python/JS/TS (gate the `dropped(ExternalReceiver)` to Rust/Go).
- `src/resolution_receiver.rs` — a `PythonReceiverTyper`/`JsReceiverTyper` beside `RustReceiverTyper`, or
  extend the Expanded classifier per-language (impl choice for the plan).
- `src/navigation/queries.rs` (+ glob/stats) — the `py_js_receiver_recovery` telemetry counters.
- `src/cpg_cache.rs` — `CACHE_VERSION` bump (resolution behavior change).
- tests: Python typed-param / constructor-local / annotation / shadow-bail / external-miss-fallthrough /
  duplicate-owner-demote; TS typed-param / `const x: Foo` / `new Foo()`; **Express/JS guard stays flat**;
  Rust/Go non-regression.

## 7. Acceptance
- **pydantic + fastapi:** `kind_exact.typed_param` / `kind_exact.constructor_local` **rise**;
  `dropped_multi_owner` + `kind_nameonly.r6_single_owner` **fall** by ≈ the owner-hit count;
  **`dropped_external_receiver` stays FLAT** (the miss→fallthrough — the critical check); canary
  `multi_target_exact_sites` byte-flat.
- **Rust (ripgrep) + Go (caddy)** call-stats **byte-identical** (the language gate + drop-on-miss).
- **Express/JS** guard flat. Tier-A `--matrix-only` 0-regr; suite green; `fmt` clean.
- Report the `py_js_receiver_recovery` hit/miss-fallthrough split + the skipped-imported count.

## 8. Pipeline
spec (this) → codex spec-review → plan → plan-review → codex-implement → acceptance → diff-review → PR →
merge on green CI. Then slice 3, then 1b.
