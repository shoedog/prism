# Phase-IP Go interface dispatch — receiver-expansion (PR-2) — DESIGN

> **rev 1 (2026-06-16).** Brainstormed design, pre dual-review. Owner decisions locked: scope =
> type-assertion + interface-typed-`var` locals (implemented) + interface-slice (sketched, behind
> the seam); precision gate = **measure-then-decide** (report first, no pre-committed tightening);
> caddy re-baseline = human-gated; per-receiver-form incremental check-ins. Dual review (codex
> gpt-5.5 xhigh + claude opus) is the next gate — they should assess architecture, the seam, fit
> with the substrate progression + `docs/prism-meta-analysis-2026-06-10.md` /
> `docs/cpg-substrate-analysis-2026-06-10.md`, weak portions, and prerequisite work.

## §0 — Context & goal

Phase-IP is type-confirmed Go receiver dispatch — the **recall** counterpart to EFT's precision.
The **embedding** half shipped (#95). The **interface FOUNDATION (PR-1)** is in review (#96,
branch `phase-ip-interface`): it resolves `r.Go()` where `r` is a P6-lite-**already-typed**
receiver (typed param / constructor local) to in-repo implementers of interface `Runner`, minted
`Exact` / `InterfaceDispatch`, via signature-confirmed structural satisfaction + RTA liveness +
a kept-Exact empty-live fallback.

**PR-2 expands which receivers P6-lite recovers a static type for**, so more interface-method
call-sites reach PR-1's dispatch engine. The engine itself does **not** change — PR-2 is "recover
more receiver types"; the dispatch core (`iface_key` → `interface_impls` → satisfaction →
fallback) is inherited unchanged.

**Why this is the increment that moves caddy.** caddy's interface recall lives in **57 diff sites
(~19 distinct sites × 3 `CaddyModule` implementers)**, and **all 57 are type-assertion receivers**
(`x.(Module).CaddyModule()`). P6-lite does not recover an asserted receiver's type today, so PR-1
is **caddy-corpus-neutral by construction**. PR-2 recovers them → the 57 sites become resolvable →
caddy interface recall can finally move (after re-adjudication + a deliberate re-baseline).

**Dependency.** PR-2 builds on PR-1 (the `interface_impls` consult + signature-confirmed
satisfaction in `src/type_providers/go.rs` / `src/resolution.rs`). PR-2 branches from
`phase-ip-interface` (or `main` once #96 merges). This is the **only** foundational dependency
(see §11).

## §1 — Scope

**Implemented:**
1. **Type-assertion receivers** — `x.(Module).CaddyModule()`. The asserted type is explicit in
   source. (All 57 caddy sites.)
2. **Interface-typed `var` locals** — `var r Runner` (and `r := …` where the declared/annotated
   static type is a known interface). PR-1 deliberately did not recover these; the
   interface-set predicate (§4) makes it safe.

**Sketched (designed-for, not implemented):**
3. **Interface-slice element receivers** — `for _, r := range runners { r.Go() }` where
   `runners []Runner`. The seam reserves this case; the implementation is stubbed. If forms 1–2
   land cleanly it is a small follow-on; if complexity balloons, **type-assertion alone is the
   floor** (it carries all the caddy value).

**Deferred-conditional** (activated only if the §8 gate *report* shows the corresponding FP/recall
class — *measure-then-decide*, not pre-committed): precise cross-package keys (D2), non-local /
factory-return liveness (D4), the fan-out width-lever. **Stays deferred:** pointer-embed promoted
admission (D5), CHA absolute-vs-relative path canonicalization (D6), Python inheritance, Rust
S3.1 struct-field-index, Go generics/type-sets, anonymous interfaces.

## §2 — The receiver-classifier seam (core abstraction)

Today receiver recovery is **two inline cases** in AST extraction (`src/ast.rs`): typed-param
recovery (`receiver_type_in_fn`, ~`ast.rs:313`) and constructor-local recovery
(`walk_receiver_bindings`, ~`ast.rs:3816`), each assigning `CallSite.receiver_type` +
`CallSite.receiver_recovery` at extraction time. PR-2 consolidates these **and** the new forms
behind one seam.

**Placement.** The classifier is the **S3 receiver-recovery component** — it lives in the S3
vocabulary (`src/resolution.rs`, alongside `ReceiverRecovery` / `peel_type` / the shared
`resolve_call_site` ladder), as a pure function **called by** AST extraction. It does not invent a
new layer and it is not a `TypeProvider` responsibility (the `TypeProvider` answers "what is type
T"; the classifier answers "what is the static type of *this receiver expression*"). Per CLAUDE.md
nav + CPG already share one ladder, so recovering richer receiver types benefits diff-review,
nav, and Tier-2 Step-5b uniformly through that single seam.

**Seam shape (owner's swap requirement).** A `ReceiverClassifier` with two implementations:
- `legacy` — the current two cases (`TypedParam` + `ConstructorLocal`), extracted **verbatim**.
- `expanded` — `legacy` ∪ the new forms (type-assertion, interface-local, [sketched] slice).

Selection is explicit (a `SliceConfig`/build flag, default `expanded`; `legacy` is the clean
fall-back). This is a **strangler seam**: we can ship `expanded`, and if a corpus regression
appears we flip to `legacy` (or `legacy` + a single new case) **without reverting commits**. Each
new form is independently gated so the fall-back is granular.

**Signature (illustrative; exact threading verified at plan time):**
```rust
// src/resolution.rs
pub struct RecoveredReceiver {
    pub static_type: String,        // bare/owner-normalized type name, as today
    pub recovery: ReceiverRecovery, // the syntactic fact that recovered it
}

pub trait ReceiverClassifier {
    /// Recover the static type of a method-call receiver, or None (unrecovered → existing
    /// drop/over-approx behavior is unchanged).
    fn classify(&self, ctx: ReceiverCtx<'_>) -> Option<RecoveredReceiver>;
}
// ReceiverCtx bundles what the two inline cases already read: the receiver expr node, the
// enclosing fn node, the call line, and Option<&GoTypeProvider> (interface set; see §4).
```

**Wire-compatibility (load-bearing per cpg-substrate "HOLD window").** `ReceiverRecovery` gains
additive variants — `TypeAssertion`, `InterfaceLocal`, and a **reserved** `SliceElem` — which is
serde/bincode-additive. `CallSite.receiver_type: Option<String>` /
`receiver_recovery: Option<ReceiverRecovery>` are **unchanged** (no struct/wire change, no forced
`CACHE_VERSION` bump from the recovery side; only the satisfaction/telemetry surface from PR-1
already moved). The **output** classification stays `ResolutionKind::InterfaceDispatch` —
`ReceiverRecovery` is the *input fact*, not a new output kind.

## §3 — Form 1: type-assertion receivers (`x.(Module).CaddyModule()`)

The call `x.(Module).CaddyModule()` is a method call whose receiver expression is a **type
assertion** `x.(Module)` — the asserted type `Module` is explicit. The classifier detects the
type-assertion node in the receiver position and recovers `Module` (owner-normalized), tagged
`ReceiverRecovery::TypeAssertion`. `Module` is then an interface name (or concrete) that flows to
the existing `iface_key` → `interface_impls` consult exactly as PR-1's typed-param path.

> Grammar note (verify at plan time): the tree-sitter-go node for `x.(T)` is the type-assertion
> expression; the outer `…​.CaddyModule()` is a `call_expression` whose `function` is a
> `selector_expression` whose `operand` is that type-assertion. The classifier walks
> receiver-expr → (type_assertion) → asserted type. The 6 canon/grammar facts PR-1 pinned
> (channel `value`, etc.) are the precedent for verifying node kinds against the pinned 0.23.4
> grammar with tests as the oracle.

## §4 — Form 2: interface-typed `var` locals (`var r Runner`)

When a local's **declared static type is a known interface**, recover it (tagged
`ReceiverRecovery::InterfaceLocal`). The safety predicate is **`declared_type ∈
GoTypeProvider.interfaces`** — the same interface set PR-1's satisfaction is built from. This is
why PR-1 could not safely do it: without the interface-set check, an untyped/concrete `var`
receiver would over-recover. With the check, only genuine interface-typed locals are recovered,
and they flow to `interface_impls` (a concrete-typed local already resolves via the existing
owner index, untouched).

Covers `var r Runner` and the annotated short-var `var r Runner = f()`; the bare `r := f()` where
`f`'s return type is an interface is **inference-dependent** and treated as the boundary toward the
sketched/deferred work (return-type inference is the D4/factory-return seam — out of PR-2 unless
the gate motivates it).

## §5 — Form 3 (SKETCH ONLY): interface-slice element receivers

`for _, r := range runners { r.Go() }` where `runners []Runner`: the range variable `r`'s static
type is the slice's element type. The seam reserves `ReceiverRecovery::SliceElem`; the classifier
case is **stubbed** (returns `None`). Designing the seam to accommodate it now (the `ReceiverCtx`
already carries the enclosing-fn scope needed to find the range source's element type) ensures the
abstraction is right; implementing it is a follow-on. **Open question for review:** does element
type come from the slice's declaration site (intra-procedural) only, or does it need light
return-type/threading inference (shared with D4)?

## §6 — Dispatch flow (unchanged, inherited from PR-1)

For every recovered interface receiver type `I` and method `m`: `iface_key(I)` → consult
`interface_impls[(I, m)]` at the P6-lite seam → if non-empty, mint N `Exact` /
`InterfaceDispatch` callees; satisfaction is signature-confirmed (`set_ptr ⊇ set_value`, promoted
methods, generics/embedded gaps); RTA prunes to the live admission-key set; the empty-live
fallback is kept **Exact**. The Go-language gate on the consult (PR-1) still applies. **No engine
change** — PR-2 only increases the population of receivers that carry a recovered `I`.

## §7 — Confidence & honesty

Inherited from PR-1 + the substrate honesty norm ("label confidence, don't delete edges"): a
recovered-interface dispatch is `Exact` **only** when signature-confirmed; otherwise the kept-Exact
empty-live fallback fires (no silent drops). PR-2 introduces no new confidence rule; the new
receiver forms reuse PR-1's confidence/kind exactly. (If the §8 report shows a recovery form is a
precision liability, the *measure-then-decide* response — width-lever / cross-package keys / a
confidence downgrade — is chosen then, not pre-committed.)

## §8 — Acceptance: the in-scope manifest + the precision gate (as a report)

PR-1 ships the per-edge `resolution_kind`/`dispatch_kind` extraction + run-JSON persistence +
replay (spec §14). PR-2 owns the two pieces PR-1 deferred:

**(a) The in-scope interface-site manifest (§14b "separate AST/drop-telemetry source").** PR-1's
manifest can only enumerate *resolved* `InterfaceDispatch` edges — it cannot see receivers that
were never recovered. PR-2 adds an **AST scan** that enumerates **every interface-method
call-site** (resolved or not), keyed `file:line`, **fingerprinted** (window-hash, reusing the
existing `adjudication.fingerprint`), **stratified by receiver class** (`TypedParam`,
`ConstructorLocal`, `TypeAssertion`, `InterfaceLocal`, `SliceElem`, `unrecovered`). This is the
denominator the resolved-edge manifest cannot produce, and the unit of the gate.

**(b) The precision gate — built as a REPORT, not a hard fail (owner: measure-then-decide).** Over
the manifest's in-scope sites, compute **interface-dispatch-attributable false positives in
`ExactOnly`**, per receiver class. PR-2 *runs and reports* it; it does **not** pre-commit to a
hard-zero bar or to any tightening (cross-package keys / width-lever). After the first real run we
read the report — what trips, and from what class — and decide the response together. (If/when we
choose to make it gating, the bar and the response are set then.)

The harness already has the pieces this builds on: `fingerprint` (`adjudication.py`), the
re-anchor map, site-fingerprint collection (`cli.py`), and `Adjudication.dispatch_kind`
(`model.py`). PR-2 adds the AST-scan manifest source + the per-class FP report.

## §9 — caddy re-adjudication + re-baseline (human-gated)

The 57 caddy sites are currently adjudicated `ambiguous` (gopls returns ambiguous interface
satisfaction; caddy `callers/C-method` recall holds at 1.000). Once PR-2 resolves them:
1. **Re-adjudicate** the 57 via the dual-adjudicator protocol (codex + claude), record Cohen's κ,
   re-anchor stale verdicts by fingerprint (the established 1:1 fingerprint match).
2. **Re-baseline** caddy — full 5-corpus rerun (`uv run tier-a --corpus all`) + a deliberate
   update of the committed anchor in `docs/eval/tier-a/`, with the adjudication record. This is
   **human-gated** (multi-corpus runs are human-triggered); the spec/plan prepares it, the owner
   runs it. **This is the PR that should move the caddy metric** — the move is recorded, not
   silent.

## §10 — Incremental check-ins (slicing)

Each receiver form is an **independent check-in behind the seam**, regardless of whether PR-2 ships
as one PR or splits:
- **Slice A:** the `ReceiverClassifier` seam + `legacy` impl (pure refactor; PR-1 recovery tests
  pin behavior — byte-identical resolution).
- **Slice B:** `TypeAssertion` form + tests + a new tier-A `interface_dispatch_assert` capability
  fixture (the caddy pattern).
- **Slice C:** `InterfaceLocal` form + tests + fixture.
- **Slice D:** the manifest source + the gate report.
- **Slice E (human-gated):** caddy re-adjudication + re-baseline.
- Slice F (`SliceElem`) is sketched only.

The seam (Slice A) landing first means B/C/D are additive and individually revertable to `legacy`.

## §11 — Substrate alignment & prerequisites

**Alignment (for the reviewers to verify).** PR-2 is the **S3 receiver-recovery component** of the
`S1→S2→S3→S4` call-resolution precision floor (`cpg-substrate-analysis-2026-06-10.md`): it tightens
the *shared* resolver (benefiting diff-review, nav, and Tier-2 Step-5b together), extends the
existing `ReceiverRecovery` vocabulary rather than a parallel one, consumes the **E12
`TypeProvider`** oracle (the GoTypeProvider interface set), and feeds the **Phase-IP**
interprocedural dispatch (PR-1). It respects the "no-build / all-text" load-bearing property
(`prism-meta-analysis-2026-06-10.md`): recovery is syntactic + optionally type-provider-confirmed,
never compiler-dependent; the `Source::ExternalIndex`/SCIP oracle remains a *future* arbitration
seam, not a PR-2 dependency.

**Prerequisites — what must be true before PR-2 begins (reviewers should pressure-test this):**
- **PR-1 (#96) merged or branched-from.** The only hard dependency — PR-2's recovered types are
  inert without the `interface_impls` consult + satisfaction.
- **S1/S2/S3/E12/embedding already merged** — confirmed in current `main` (S2 `dd60ed6`; the
  shared `resolve_call_site` ladder; `GoTypeProvider`; #95). The 2026-06-10 docs predate these
  landings; their "S1/S2 pending" framing is the stale snapshot.
- **Open prerequisite question for review:** does the manifest's AST-scan source (§8a) want any S2
  node-identity affordance (byte-range site keys) that isn't already present, or is `file:line` +
  fingerprint sufficient? And should Slice A (the seam refactor) land as its own PR *before* the
  new forms, to de-risk the strangler swap?

## §12 — Testing

- **Classifier unit tests** (per form): `TypeAssertion` recovers the asserted type;
  `InterfaceLocal` recovers iff declared type ∈ interface set (and does **not** recover a
  concrete-typed local); `SliceElem` is an `#[ignore]`d placeholder pinning the reserved variant.
- **`legacy` parity test:** `legacy` classifier reproduces PR-1's recovery byte-for-byte on the
  existing P6-lite fixtures (the strangler-swap guard).
- **Resolution tests:** `x.(Module).CaddyModule()` and `var r Runner; r.Go()` → in-repo
  implementers, `Exact` / `InterfaceDispatch`; non-interface asserted/var receivers unaffected;
  the Go-language gate still blocks cross-language.
- **tier-A fixtures:** `go/interface_dispatch_assert` (+ `_var`) capability fixtures mirroring the
  caddy 57-site pattern.
- **Harness:** manifest-source enumeration/stratification test; gate-report computation test.

## §13 — Open decisions (resolved via measure-then-decide / review)

1. Slice-element type source (§5): declaration-site only vs. light return-type inference.
2. Gate response if it trips (§8b): cross-package keys vs. width-lever vs. documented threshold —
   decided after the first report.
3. Should Slice A (seam refactor) be its own pre-PR (§11)?
4. `expanded` vs `legacy` default selection + how the flag is surfaced (build flag vs `SliceConfig`).

---

### Self-review (author)

- **Placeholders:** none. The illustrative `ReceiverClassifier` signature is explicitly "verified
  at plan time"; AST node kinds are flagged "verify against grammar" (the PR-1 precedent).
- **Consistency:** `ReceiverRecovery` = input fact (extended additively); `ResolutionKind` =
  output (unchanged, stays `InterfaceDispatch`) — stated identically in §2/§6. Scope (§1) matches
  the slices (§10) and tests (§12).
- **Scope:** one implementation plan's worth — seam + 2 forms + manifest/report + (human-gated)
  re-baseline; slice sketched; tightening deferred-conditional. Not over-bundled.
- **Ambiguity:** the precision gate is a **report** (not gating) in PR-2 — stated explicitly to
  avoid the "is it hard-zero?" reading. The caddy re-baseline is **human-gated** — stated.
