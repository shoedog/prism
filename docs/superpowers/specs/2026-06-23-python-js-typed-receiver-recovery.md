# Python/JS Typed-Receiver Recovery — Design (2026-06-23, rev 3)

> Slice 2 of the Python/JS resolution-maturity loop (after 1a #131 + decorated #132). Basis: codex
> architect memo + spec-review. Branch `slice2-typed-receivers` off merged main.
>
> **Rev 2 — codex spec-review fold (REWORK):** (BLOCKER) the miss-behavior was self-contradictory →
> replaced with an explicit **triage** (§3.2). (BLOCKER) the external-collision guard must check the
> **recovered TYPE name**, not the receiver var `q` (§3.3). (MAJOR) acceptance + a **singleton** external-
> vs-local test, since the canary is blind to one wrong Exact (§7); the R6 recovered branch **returns**
> before residue — the plan must explicitly "not return on Python/JS miss" (§3.2); JS `x = Foo()` is a
> factory not a constructor → JS/TS use **`new_expression`** only, `Foo()`-constructor stays Python-only
> (§3.1). (NIT) gate is `src/ast.rs:424`.
>
> **Rev 3 — codex spec re-review fold (REWORK):** (BLOCKER) Python `from X import *` wildcard imports are
> NOT in the imports map → the §3.3 type-name guard missed them (singleton false-Exact still constructible)
> → added a **wildcard sentinel** that skips recovery for any non-same-file-local type when a wildcard is
> present (§3.2.1, §3.3) + a wildcard test (§7). (MINOR) §3.2.2 clarified: `owner_lookup` miss conflates
> known-local-lacks-method and no-owner-key — both fall through. Verdicts 1,3,4,5 were TRUE
> (triage / R6-no-return / JS-`new` / Rust-Go byte-id all sound).

## 1. Problem
`x.method()` where `x`'s static type is syntactically recoverable (typed param `def f(x: Foo)` / TS
`f(x: Foo)`; Python constructor local `x = Foo()`; JS/TS `x = new Foo()`; explicit annotation `x: Foo`) is
unresolved/NameOnly-demoted for Python/JS/TS. The P6-lite recovery is `Rust|Go`-gated at
`receiver_type_in_fn` (`src/ast.rs:424`) and `recover_simple_ident` (`src/resolution.rs:320`); Rust/Go feed
the type to R6 `owner_lookup(recv_ty, name)` (`src/resolution.rs:~1110`) → `TypedParam`/`ConstructorLocal`.
**Buy:** ~171 FastAPI + ~542 pydantic recoverable owner-hit sites (opportunity denominator — realized Exact
is less after demote-on-multi + the triage; Express ≈0).

## 2. Goal
Recover the receiver static type for Python/JS/TS and resolve `x.method()` to the in-repo owner's method,
**without** minting a false Exact to a same-named in-repo class when the real type is external, **without**
a `dropped_external_receiver` spike, and **byte-identical for Rust/Go**.

## 3. Mechanism

### 3.1 Recovery (what produces a `static_type`)
Open the gates for `Python|JavaScript|TypeScript|Tsx` and recover, reusing the one-binding+shadow-bail scan:
- **Typed params:** Python `typed_parameter`/`typed_default_parameter` (`type` field); TS parameter
  `type_annotation` (strip the leading `:` — see `type_providers/typescript.rs:277-283`).
- **Constructor locals:** Python `x = Foo()` (a `call` whose function is a bare class-like name) — **Python
  only**; JS/TS `x = new Foo()` (`new_expression`) — **NOT** bare `Foo()` (that's a factory call, unsound).
- **Explicit annotations:** Python `x: Foo` annotated assignment; TS `const x: Foo`.

### 3.2 Triage (BLOCKER-1 fix — the miss-behavior, replaces the old contradiction)
After recovery, peel + owner-key the type to `T`, then:
1. **`T` is import-bound OR shadowable by a wildcard** (its name is in the file `imports` map, OR — rev-3
   BLOCKER fix — the file has a Python `from X import *` wildcard AND `T` is **not** a class defined in the
   same file) → **do NOT recover** (skip; leave `receiver_type` unset) → normal R6 residue as today.
   (Imported/wildcard-sourced types are external or cross-module — slice 3/4; recovering them risks the
   §3.3 false-Exact.)
2. **`T` survives the guards**, `owner_lookup(T, name)`:
   - **hit** → relabel `TypedParam`/`ConstructorLocal` Exact (the buy).
   - **miss** — `owner_lookup` returns None, which conflates *known-local-owner-lacks-method* AND
     *no-owner-key-for-`T`* (the resolver has no separate "known local owner" index); **both** → for
     `Python|JS|TS|Tsx` **fall through to the R6 residue** (do **NOT** `return dropped(ExternalReceiver)`).
     **Rust/Go keep drop-on-miss (byte-identical).**

**Implementation note (MAJOR fix):** the R6 recovered-receiver block currently early-`return`s on miss
(`src/resolution.rs:~1117`/`~1162`). The plan MUST restructure so a Python/JS/TS miss does **not** return —
it continues into the residue path (`~:1166`). Gate the existing `dropped(ExternalReceiver)` to
Rust/Go.

### 3.3 External-collision guard (BLOCKER-2 fix)
The skip-on-import check must be on the **recovered type name `T`**, NOT the receiver qualifier `q` (the
existing `recover_simple_ident` checks `q` against imports — wrong for `def f(x: Foo)` where `x`≠import but
`Foo`=import). Add a **post-recovery** guard: skip recovery if the peeled type name is in the file `imports` map
(triage §3.2.1). **Rev-3 BLOCKER fix — wildcard:** Python `from X import *` does NOT add the exported
names to the imports map, so the import check alone misses it. Record a per-file **wildcard sentinel**
(detect the `wildcard_import` child of `import_from_statement`); when present, skip recovery for any type
name **not proven to be a same-file class definition** (a wildcard cannot shadow a same-file `class T`). Residual (accepted, documented + tested): a type defined locally that *shadows* an
external of the same name; and over-skip of in-repo *imported* types (recall cost — quantified by
telemetry). This is the sound first-merge floor.

### 3.4 Telemetry
`py_js_receiver_recovery { hit, miss_fallthrough, skipped_imported }` in call-stats.

## 4. Soundness
- Multi-owner: `owner_lookup` demotes (`resolution.rs:773`) → no multi-owner wrong-Exact.
- Singleton false-Exact (external `T` colliding with one local `T`): prevented by §3.3 (skip imported `T`).
  The only residual is a locally-defined `T` shadowing an external `T` (rare) — documented, tested.
- Rust/Go byte-identical (language gate + preserved drop-on-miss).

## 5. Scope
**In:** open the 2 gates Python/JS/TS; recover typed-params + constructor-locals (`Foo()` Python /
`new Foo()` JS-TS) + explicit annotations of **local** classes; the §3.2 triage + §3.3 type-name guard +
miss→fallthrough (Python/JS, do-not-return); telemetry; tests. **Out:** imported/cross-module type
resolution (slice 3/4), Python `self.field: Foo` field types, TS structural/interface typing,
CommonJS/prototype/factory typing, chained-receiver, span-keyed typed identity.

## 6. Files
- `src/ast.rs` — `receiver_type_in_fn` (`:424` gate + Python/JS/TS param/annotation/constructor cases;
  JS/TS constructor = `new_expression` only); `walk_receiver_bindings` (`~:3955-4074`) Python/JS arms;
  constructor recovery currently Rust/Go-only at `~:4106-4126`.
- `src/resolution.rs` — `recover_simple_ident` (`:320` gate); the **post-recovery type-name import guard**;
  R6 (`~:1110-1166`) miss→fallthrough for Python/JS/TS (do-not-return; `ExternalReceiver` drop → Rust/Go).
- `src/resolution_receiver.rs` — `PythonReceiverTyper`/`JsReceiverTyper` or per-language Expanded arms.
- `src/navigation/queries.rs` (+ stats) — telemetry. `src/cpg_cache.rs` — `CACHE_VERSION` bump.
- tests (see §7).

## 7. Acceptance
- **pydantic + fastapi:** `kind_exact.typed_param`/`constructor_local` **rise**; `dropped_multi_owner` +
  `kind_nameonly.r6_single_owner` **fall**; **`dropped_external_receiver` byte-FLAT** (the triage/fallthrough);
  canary `multi_target_exact_sites` byte-flat.
- **Singleton false-Exact test (MAJOR):** `def f(x: Foo): x.m()` with `Foo` IMPORTED externally + a single
  in-repo `class Foo` with `m` → must **NOT** Exact-bind to the in-repo `Foo.m` (skipped via §3.3).
- **Wildcard false-Exact test (rev-3):** `from ext import *` + `def f(x: Foo): x.m()` + a single in-repo
  `class Foo` (in ANOTHER file) → must **NOT** Exact-bind (wildcard sentinel skips; `Foo` is not same-file).
- **Triage tests:** local typed-param hit → Exact; local-miss → residue (not dropped); imported type →
  skipped (residue); shadow-bail; `new Foo()` JS hit; bare `Foo()` in JS → NOT recovered.
- **Rust (ripgrep) + Go (caddy)** call-stats **byte-identical**. Express/JS guard flat. Tier-A
  `--matrix-only` 0-regr; suite green; `fmt` clean. Report the telemetry hit/miss/skipped split.

## 8. Pipeline
spec rev 2 → codex spec re-review → plan → plan-review → codex-implement → acceptance → diff-review → PR →
merge on green CI. Then slice 3, then 1b.
