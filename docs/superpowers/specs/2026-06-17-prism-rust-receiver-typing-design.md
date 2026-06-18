# Receiver-Typed Method Resolution — Design Spec (Rust first; Go-consistent)

> **Status: CONVERGED (rev 7) — codex gpt-5.5 xhigh round 6 ruled PLAN-READY; rounds 1–5 FLAWED→revised, round 6 SOUND-WITH-CONCERNS (2 wording folds).**
> Design-of-record candidate for **Phase 2** of prism's name-resolution arc: resolving `x.method()` by the
> **static type of the receiver**. Builds on the Phase-1 scope graph (name resolution — MERGED:
> #102/#103/#104/#105) and the proven Phase-IP **Go** receiver dispatch (embedding #95 + interface
> satisfaction #96/#97 + arity-disambiguation #100). **HELD before plan/development pending the owner's
> go-for-slicing** (spec → codex gpt-5.5 xhigh review rounds → SOUND → plan → implement, same arc as Phase 1).
>
> **Standard (owner):** comprehensive schematic up front, no naive approximation. The *model* must represent
> the full receiver-typing problem (resolved type identity, the autoderef candidate chain with
> receiver-place + applicability, inherent-vs-trait priority, trait-in-scope, extension traits);
> *populating/consuming* it is phased. Recall-safety is structural: **resolve-or-fall-through, never a wrong
> method edge.** A change that REMOVES a today-minted edge is Tier-A-gated; an additive or
> sound-by-construction change is not.
>
> **rev-6 thesis (rounds 4+5+6 corrections):** **Phase 2a preserves the live branch shape; its only
> edge-removing change is a sound, Tier-A-measured drop-on-miss.**
> It preserves the live branch shape — std-wrapper peel, recovered→`owner_lookup`→drop-on-miss, and "only
> *unrecovered* receivers hit the residue" (verified: `resolution.rs:781-835` returns in all branches; residue
> at `:837` fires only when `receiver_type` is `None`). Two refinements make the additive claim *true* (round 5):
> (i) the 2a combine uses **`MethodFacts.kind`** — **inherent-single → Exact; trait-impl/wrapper-peeled-single
> → NameOnly-demote** — so newly field/return-recovered trait/wrapper receivers land at honest NameOnly, never
> a wrong Exact; (ii) **qualified identity is materialized only when BOTH the impl-header and the receiver
> resolve confidently to an identity, else it falls back to today's bare-string `owner_lookup`** — so a
> currently-correct bare-key edge is never removed. The 2a net adds: more recovered receivers (recall, as
> Exact-inherent / NameOnly-trait), best-effort cross-module disambiguation (precision, never-worse), and
> external-type-keyed extension-trait indexing — all non-removing. The **one** edge-removing 2a lever is the
> **drop-on-miss for newly-recovered receivers** (where the receiver-method FPs die): sound (the dropped
> residue over-claim was wrong) but, per the owner's standard, **Tier-A-measured in the 2a gate** — not a free
> lunch. Further subtractions (residue removal for *unrecovered* receivers; raising trait/wrapper
> NameOnly→Exact) are **gated Phase-2b/3**. The model still represents the whole problem.

---

## §0 Review response

### §0.1–0.3 Rounds 1–3 (all FLAWED → folded in revs 2–4; full tables in git history)
- **Round 1:** "too string-keyed/owner-only" → resolved model, `MethodFacts`, the `rust_provider` discovery.
- **Round 2:** "concepts right, folds asserted not code-supported" (**Go fold SOUND**) → resolved-`ScopeId`
  `TypeKey`, def-byte binding key, CallGraph-resident indices.
- **Round 3:** "materially better" (cfg + AST-`MethodFacts` **SOUND**) → the build-time materialization spine;
  caught rev-3's over-correction (inherent-only would regress the trait fixtures).

### §0.4 Round 4 (FLAWED → folded in **this rev 5**)
Round 4 confirmed **#6 (build-time materialization) FOLDED-SOUND** and **R5/R7 SOUND**, but caught three real
Rust-semantics bugs in rev-4 — all verified against the live code and fixtures:

| # | Round-4 finding (verified) | Severity | Resolution in rev 5 |
|---|---|---|---|
| F1 | **External-drop is unsound — extension traits.** `impl MyExt for String { fn m }` is an *in-repo* method on a *foreign* receiver (`languages/mod.rs:1011`, indexed under the receiver type via `method_metadata` `call_graph.rs:1340`). Dropping every `External` receiver before lookup loses correct edges. | BLOCKER | §2.2/§3.3: External is **NOT a pre-lookup drop**. `owner_lookup(TypeKey, m)` runs for External receivers too (extension impls are keyed under the external type — confirmed) and **drops only if the lookup is empty**. This is exactly today's "recovered→lookup→drop-on-miss" behavior; rev 4's "drop before lookup" is removed. |
| F2 | **"Single in-repo candidate → Exact" is overclaimed.** A single in-repo trait impl can be non-callable at the site while the real target is a std/prelude trait or a Deref-target prism can't see (`resolution.rs:595-603` returns Exact on one primary owner, with no trait-in-scope/mutability/autoderef facts). | BLOCKER | §2.2/§9: rev 5 **does not claim soundness** — it states 2a **preserves today's combine** (single→Exact incl. trait, exactly the current behavior, no regression) and names the known imprecision (out-of-scope/shadowed trait, Deref-target) as a **Phase-3 precision refinement** (inherent→Exact, trait→demote unless trait-in-scope + unshadowed), Tier-A-gated. The false "provably sound" claim is deleted. |
| F3 | **Wrapper → `Unrecoverable` is edge-removing.** Today typed-param recovery peels `Box/Arc/Rc/Pin` (`resolution.rs:184-188`) and resolves via `owner_lookup` (`:319-323`); rerouting to the residue removes those edges. | BLOCKER | §2.1: rev 5 **preserves the current wrapper peel** (peel → `owner_lookup` → resolve/drop, as today). The `Arc::clone`-class precision risk is a **pre-existing** behavior, refined in Phase 3 (wrapper/Deref-aware dispatch), not changed in 2a. |
| F4 | `resolve_type_path_to_type_scope` under-specified — must distinguish bare-lexical names (lexical `resolve`) from anchored/multi-segment paths (`resolve_path`), imports, aliases, and external paths; external classification (`std/core/alloc`) lives only in the module-deps consumer (`consumer.rs:123-135/175-180`). | MAJOR | §3.1: the helper is specified to branch — bare single-segment → lexical `resolve` in NS_TYPE; anchored/multi-segment → `resolve_path`; alias → resolve target; `std/core/alloc`/dep leading segment or unresolved → `External(name)`. (Plan-time: exact API.) |
| F5 | Build-time binding lookup needs a **direct** graph helper — `resolve` does not return the `Binding`/index (`Candidate.provenance` empty, `types.rs:443-450`). | MAJOR | §3.2b: the typer calls a **direct visible-binding lookup** (the rib search) returning the `Binding`/`Span`, not "`resolve` then read the binding." (Plan-time: the helper.) |
| F6 | `ScopeId` is serializable but **not content-stable across rebuilds** (`types.rs:24-29`); the incremental path merges stale call sites before rebuilding the scope graph (`cpg/build.rs:222-234`). | MINOR | §8: the receiver `TypeKey`s, indices, and scope graph are **rebuilt together**; incremental rebuild must **re-materialize all receiver outcomes** (not merge stale call sites). (Plan-time.) |

**Net:** Phase 2a is now provably **non-regressing** — it preserves the combine rule, the wrapper peel, and
the recovered→drop-on-miss / unrecovered→residue split, all confirmed as today's behavior. Its precision win
comes from **recovering more receivers** (so they reach the existing sound drop-on-miss path) + **qualified
identity** (no cross-module wrong hits). Every subtractive precision change is a gated later phase.

### §0.5 Round 5 (FLAWED → folded in **this rev 6**)
Round 5 confirmed **F1/F3/F5/F6 FOLDED-SOUND** but caught that rev-5's "additive by construction" was overstated
in two ways. Both verified against source and folded:

| # | Round-5 finding (verified) | Severity | Resolution in rev 6 |
|---|---|---|---|
| G1 | **New recovery is not additive while preserving single→Exact.** A call *unrecovered* today gets `R6SingleOwner` **NameOnly** (or drops); field/return recovery moves it into `owner_lookup` where a single **trait-impl/wrapper** candidate becomes **Exact** (`resolution.rs:595-603`/`:781-797`) — an Exact upgrade on a possibly non-callable (out-of-scope) trait method or std-shadowed wrapper target. | BLOCKER | §2.2/§3.3: the 2a combine uses **`MethodFacts.kind`** — **inherent-single → Exact**; **trait-impl-single / wrapper-peeled-single → NameOnly-demote**. Verified non-regressing: the capability fixtures assert the **caller-set** (`matrix.py:76` `got == expect_callers`), and NameOnly edges are included, so `trait_static_dispatch`/`trait_dyn_dispatch` still pass. New field/return trait/wrapper recovery lands at honest NameOnly, never wrong-Exact. Raising NameOnly→Exact (trait-in-scope/applicability) is Phase 3. |
| G2 | **Qualified identity is not "only drops wrong hits."** If impl-header *or* receiver resolution fails asymmetrically (populator nested-item/macro/cfg/alias gaps), re-keying bare `String`→`ScopeId`/`External` can **miss a currently-correct bare-key edge**; "unresolved ⇒ External" then drops it. | BLOCKER | §2.3/§3.1: **identity-or-fall-through-to-bare** — materialize the resolved `TypeKey` **only when BOTH the impl-header type and the receiver type resolve confidently** to an identity; otherwise **fall back to today's bare-string `owner_lookup`** (all-or-nothing, legacy fallback). `External(name)` is used **only for confidently-external** (std/core/alloc/dep leading segment), never as a catch-all for "unresolved." A correct bare-key edge is never removed; cross-module disambiguation is best-effort, never-worse. |
| G3 | **`External` keys need canonicalization** — extension impls survive only if `String`, `std::string::String`, `alloc::string::String`, and aliases map to one key; `owner_key` collapses by stripping paths today. | MAJOR | §3.1: `External(name)` uses a **canonical key** (last-segment normalization + a known-std-type canonical map) applied identically to impl-header extraction and receiver typing, so extension impls and their receivers agree. |

**Net of round 5:** inherent-method recovery (the safe majority) gains Exact edges; trait/wrapper recovery
gains NameOnly edges; identity falls back to bare on partial resolution. Spec-time blockers cleared.

### §0.6 Round 6 (the plan-readiness GATE) — **VERDICT: SOUND-WITH-CONCERNS → PLAN-READY**
Round 6 verified G1/G2/G3 **FOLDED-SOUND** against the live harness (`matrix.py:74-77` compares caller-sets,
`sut.py:188-200` defaults confidence "all", `queries.rs:407-410` drops NameOnly only under exact-only) and
ruled **"PLAN-READY. No remaining spec-time blocker."** Two wording/honesty findings, folded in **this rev 7**:

| # | Round-6 finding | Severity | Resolution in rev 7 |
|---|---|---|---|
| 1 | "2a removes no edge / only confidence is refined" is **overbroad** — field/return recovery + empty-lookup *intentionally removes* today's R6 residue edges (`resolution.rs:837-906`) when the receiver is now certainly typed with no in-repo `m`. Sound precision, but **edge-removing → must be Tier-A measured/gated**. | MAJOR | §2.3/§7/§9 reworded: 2a has **two non-removing levers** (confidence refinement, identity-or-bare) **plus one edge-removing lever** — the **drop-on-miss for newly-recovered receivers** — which is sound but **Tier-A-measured in the 2a gate** (where the receiver-method FPs die), not a free "no edge removed." |
| 2 | §7 "trait-single demote refinement" wording-confusing (demotion is 2a; raising is Phase 3). | MINOR | §7/§9 reworded: **2a demotes** trait/wrapper-single to NameOnly; **Phase 3 raises** NameOnly→Exact. |

**Net: the spec is converged and PLAN-READY** (rev 7). The model represents the full problem (applicability,
trait-in-scope, wrapper/Deref dispatch, extension traits, chains) with phased consumption; Phase 2a is a
non-regressing-except-a-measured-drop-on-miss increment. Carry-forward **plan-time** items: F4 (the
`resolve_type_path_to_type_scope` branching), F5 (the direct visible-binding lookup helper), F6
(rebuild-together / re-materialize-on-incremental), G2 (the per-bare-bucket "identity complete" guard), and G3
(the `External` canonical-map mechanics).

---

## §1 Goal, the model, the seam

**Goal:** resolve `x.method(args)` to the **single in-repo method** the receiver's static type selects — or
**fall through** (the existing residue / drop) when the type/target can't be established with certainty. The
#1 remaining Rust precision *and* recall lever (the 2026-06-17 re-anchor):
- **Precision:** the standing `prism_fp` over-claims are receiver/type-blind. The **receiver-method** FPs
  (`.target()`/`.edges()`/`.truncate()` on an external value) arise because the receiver is **unrecovered**
  today (P6-lite recovers only typed-params + constructor-locals), so the call falls to the **residue**, which
  mints a name-based edge to a same-named in-repo method. The **cross-module** FPs (`a::Foo`/`b::Foo`) arise
  because owner keys are bare strings. Recovering more receivers (so they reach the existing drop-on-miss
  path) + qualified identity kill both — additively.
- **Recall:** the bulk of `oracle_only` (prism-missing) edges are method calls where prism recovers no
  receiver type, so `owner_lookup` finds nothing and the residue (a weak guess) is all there is. Recovering
  more types (field/return) resolves them precisely.

**The model = a per-language `ReceiverTyper` (syntactic, recall-safe, build-time) producing an ordered
candidate chain of RESOLVED type identities, feeding the existing — preserved — `owner_lookup` dispatch.**
Phase 1 gave *name* resolution; Phase 2 adds the *type*. They compose at **build time** (where the AST + graph
+ spans are present): name → binding (scope graph) → binding `Span`/facts → static type **candidate chain**
(resolved `TypeKey`s) → `owner_lookup` (**the kind-aware combine: inherent-single→Exact, trait/wrapper-single→NameOnly, multi→TraitCha-demote,
empty→drop**) → arity-filter. The result is **materialized onto the `CallSite`**; `resolve_call_site` reads it.
The typer is **syntactic** — no type inference, no full autoderef/autoref, no trait-bound solving — recovering
a type only when the syntax makes it certain, and falling through otherwise.

**Build-time materialization (round-3 #6, FOLDED-SOUND).** `resolve_call_site` sees only a serialized
`CallSite`. The typer runs in the **`CallGraph` build pass** (the existing P6-lite `receiver_type_in_fn` at
`resolution.rs:320` already runs there with the `ParsedFile`) and **materializes** the resolved receiver
`TypeKey` + outcome onto the `CallSite` (a serde-additive field, extending today's `receiver_type:
Option<String>` at `call_graph.rs:536/1268`, read at `resolution.rs:781`). The identity-keyed method index is
also serialized on `CallGraph`; `resolve_call_site` reads both — a pure read.

**Two identity commitments:**
- **Type identity = a resolved `TypeKey`** — the **defining type's scope id** (or `External(name)`), obtained
  by a build-time `resolve_type_path_to_type_scope` (§3.1), applied to impl-header type paths (so methods key
  under the defining type, not the impl module — round-3 R1) AND recovered receiver type names. `a::Foo` ≠
  `b::Foo`. **Extension-trait impls on external types key under `External(name)`** so `owner_lookup` finds them
  (round-4 F1).
- **Binding identity = the binding's `Span` (`FileId`, `def_byte`)**, read at build time via a **direct
  visible-binding lookup** (round-4 F5).

**The seam (extends, preserves):** `ReceiverClassifier` (`resolution.rs:250`) gains a build-time
`RustReceiverTyper`; `owner_lookup`/`owner_lookup_in_modules` (`:547/595`) gain an identity-keyed index **with
the kind-aware combine (§2.2) and a bare-string fallback**; `arity_filter` (`:153`, #100) applies to Rust; the indices are materialized on
`CallGraph` (serialized, cache-versioned) via an **identity-aware extractor** refactored from
`type_providers/rust_provider.rs` (round-4 R5: real new work, not a key-swap). `type_db.rs` stays C/C++-only.

§4 specifies Rust in full; §5 pressure-tests Go + C++/Python/TS.

---

## §2 The problem space (what the model is checked against)

✦ = correctness-critical for common Rust; Phase-2a-correct (typed, **preserved**, or strict fall-through).

### §2.1 Receiver forms (where does `x`'s type come from?), ranked by recovery certainty
- ✦ **`self` / `Self`** → the caller method's owner `TypeKey`. Highest certainty.
- ✦ **typed param** `fn g(x: T) { x.m() }` → `T` *(P6-lite: done)*; `x: &dyn Tr` → the trait's `TypeKey`
  (preserves `trait_dyn_dispatch`).
- ✦ **type-annotated let** `let x: T` → `T`.
- ✦ **constructor local** `let x = T::new(…)` / `T{…}` → `T` *(P6-lite: done)*.
- ✦ **field-typed** `let x = obj.f; x.m()` / `self.f.m()` → the field index for `(typeof(obj), f)`. **NEW
  (recall).**
- ✦ **return-typed** `let x = g(…); x.m()` / `g().m()` → the fn-return index for `resolve(g)` (`Self`→owner;
  cycle-capped §4.6). **NEW (recall).**
- ✦ **std-wrapper receiver** `x: Box<T>`/`Arc<T>`/`Rc<T>`/`Pin<T>` → **peel to `T` and `owner_lookup` — exactly
  as today** (`resolution.rs:184-188/319-323`). **PRESERVED** (round-4 F3); the `Arc::clone`-class precision
  risk is a pre-existing behavior refined in Phase 3 (wrapper/Deref-aware dispatch).
- **method-chain return** `a.b().c()` → recurse the return index — **Phase 3**.
- **uncertain → fall through (the existing residue/drop):** generic-type-param receiver, closure /
  `collect()` / inference, unresolvable. These stay exactly as today (unrecovered → residue).

### §2.2 The method target — PRESERVE the existing `owner_lookup` combine
`owner_lookup(T, m)` over the identity-keyed index, with the **unchanged** combine
(`resolution.rs:595-603`, generalized to identity keys), runs for **every** recovered receiver — in-repo or
External:
- ✦ **single in-repo candidate, by `MethodFacts.kind` (round-5 G1):**
  - **inherent** `impl T { fn m }` → **Exact** (sound: an inherent method has priority over trait methods and
    Deref-target methods at the receiver's own type — if `T` has an inherent `m`, `x.m()` calls it).
  - **single trait-impl** on `T` / **single impl of a `dyn Tr`'s trait** / **single extension impl** on an
    external `T` / **wrapper-peeled** single candidate → **NameOnly-demote** — *not Exact*, because prism
    cannot yet prove the trait is in scope or that no std/prelude/Deref target shadows it. The edge is present
    (recall preserved); the confidence is honest. Verified non-regressing: the capability fixtures assert the
    **caller-set** (`matrix.py:76`), and NameOnly edges are included, so `trait_static_dispatch` /
    `trait_dyn_dispatch` still pass. **Phase-3** raises these to Exact via trait-in-scope (NS_TYPE resolution
    from the call scope) + applicability + wrapper/Deref awareness.
- ✦ **multiple in-repo candidates** → **TraitCha demote** (NameOnly) — today's recall-safe floor. Phase-3
  raises by trait-in-scope + applicability.
- ✦ **empty lookup** (recovered receiver, no in-repo method `m` — including a recovered `External` receiver
  with no in-repo extension impl) → **drop** (today's `ExternalReceiver` path, `resolution.rs:833`). **Sound:**
  a certainly-recovered receiver whose type has no in-repo `m` cannot have an in-repo edge; the residue would
  mint a wrong one. This is where the **receiver-method FPs die** once field/return recovery (§2.1) makes
  those receivers *recovered* instead of unrecovered.
- ✦ **associated function (no `self`)** `T::new()` is **not** an `x.m()` target — `MethodFacts.has_self`
  excludes it. (An R1 qualified-call target.)
- ✦ **arity mismatch** → arity-filter drops, `arity_excl_self`, after the candidate set, on a confident exact
  mismatch only (#100/#10).
- **(Phase-3) applicability** (`ReceiverPlace`) raises/keeps multi-candidate precision; **not in 2a** (2a
  makes no inherent-vs-trait *choice* → applicability is not needed — round-4 F2/R3).

### §2.3 The precision/recall stance (two non-removing levers + one measured drop-on-miss; further subtractions gated)
2a preserves the recovered-vs-residue split and **refines confidence** via `MethodFacts.kind`. Two levers are
non-removing by construction; one (the drop-on-miss) is sound but edge-removing and Tier-A-measured (round-6
finding 1):
- **(a) field/return recovery** → more receivers recovered. A recovered receiver routes to `owner_lookup` →
  **inherent-single Exact** (recall add, high confidence) / **trait-or-wrapper-single NameOnly** (recall add,
  honest confidence — never a wrong Exact, G1) / **drop-on-miss** (the one edge-removing lever: replaces a
  residue over-claim with a drop when the receiver is certainly typed with no in-repo `m` — sound but
  **Tier-A-measured**, round-6 finding 1). No newly-recovered site produces a *wrong* edge; the drop-on-miss is
  where the receiver-method FPs die.
- **(b) qualified identity — best-effort, never-worse (G2).** Materialized only when **both** the impl-header
  and the receiver resolve confidently to an identity; otherwise **fall back to today's bare-string
  `owner_lookup`**. So it drops only *wrong* cross-module hits when it can prove them, and never removes a
  correct bare-key edge it can't fully resolve.
- **(c) extension-trait indexing** under canonical `External(type)` → adds in-repo extension edges (never
  removes).
- **Gated (Phase 2b/3):** removing/tightening the **residue** for still-*unrecovered* receivers (the pinned
  `eval/fixtures/rust/r6_single_owner_demote/`: `let x = mystery(); x.frobnicate()` is a correct demoted edge
  field/return typing can't recover — Tier-A-gated); **raising trait/wrapper-single NameOnly → Exact** via
  trait-in-scope + applicability + wrapper/Deref awareness (Phase 3). Each is Tier-A-measured.

§7 makes this an invariant; §9 sequences it.

---

## §3 The receiver-typing core

### §3.1 `RustReceiverTyper` (build-time, syntactic; resolved-identity, ordered; preserves the peel)
```
trait ReceiverTyper {                                   // runs at CallGraph BUILD time (has ParsedFile+graph)
    fn type_of_receiver(&self, ctx: ReceiverTypeCtx) -> Option<Vec<TypeCandidate>>;   // None = fall through
}
struct TypeCandidate { key: TypeKey, via: ReceiverRecovery, place: ReceiverPlace }   // place: Phase-3 applic.
enum TypeKey { InRepo(ScopeId), External(String) }      // ScopeId = the DEFINING type's scope
struct ReceiverPlace { form: Owned|Ref|RefMut, mutable_place: bool }   // modeled; consumed in Phase 3
enum ReceiverRecovery { SelfTy, TypedParam, TypedLet, ConstructorLocal, FieldTyped, ReturnTyped, StdWrapperPeel,
                        /*Phase3:*/ ChainReturn /*; Go partition keeps TypeAssertion, VarDecl, SliceElem*/ }
```
`ReceiverTypeCtx` carries the receiver node + place form, the enclosing fn node, the call byte/line, the
`ParsedFile`, and the scope graph. The typer resolves `x` to its binding via a **direct visible-binding
lookup** and reads the `Span` (→ `def_byte`) at build time (round-4 F5). **shadow-bail** (>1 candidate
binding) and **poison-respect** → `None` (fall through). A `Box/Arc/Rc/Pin` receiver is **peeled to the inner
type as today** (recovery `StdWrapperPeel`) — preserved, not dropped (round-4 F3). A generic-param / closure /
inference receiver → `None` (the unchanged residue). `None` ⇒ the site is left exactly as today.

**Type identity (round-3 R1, round-4 F4, round-5 G2/G3).** `resolve_type_path_to_type_scope(from_scope,
type_syntax) -> Option<TypeKey>` branches: a **bare single-segment** name → lexical `resolve` in NS_TYPE; an
**anchored / multi-segment** path → `resolve_path`; an **alias** → resolve its target; a **`std`/`core`/`alloc`
or known-dep leading segment** → `External(canonical(name))`. It is applied to impl-header type paths (→
methods key under the defining type's `Target::Item{owns: type_scope}`, not the impl module — round-3 R1) and
to recovered receiver names. The `dyn Tr` form resolves the **trait** path (the trait is an item) so the trait
dual-key fires.

- **Identity-or-fall-through-to-bare (round-5 G2):** the resolved `TypeKey` is used to key/look up **only when
  BOTH the impl-header type (at index-build) and the receiver type (at the call) resolve confidently** to an
  identity. If *either* fails to resolve (a populator gap — nested items, macros, cfg, unhandled alias), the
  resolver returns `None` and the path **falls back to today's bare-string `owner_lookup`** — the exact
  current behavior, so no correct bare-key edge is ever removed. `External` is **never** a catch-all for
  "unresolved": an unresolved name → `None` → bare fallback, NOT `External(name)`. `External(_)` is minted
  only for a *confidently* std/core/alloc/dep leading segment.
- **Canonical `External` key (round-5 G3):** `canonical(name)` normalizes `String` / `std::string::String` /
  `alloc::string::String` / aliases to one key, applied **identically** to impl-header extraction and receiver
  typing, so an extension impl (`impl Ext for String`) and a `String` receiver agree on the same key.

### §3.2 The Rust type indices — CallGraph-resident, identity-keyed, via an identity-aware extractor
The indices live **on `CallGraph`** (serialized, `CACHE_VERSION`-bumped) — the Phase-1 `scope_graph` precedent
(`call_graph.rs:899`). Populated in the build pass by an **identity-aware extractor** refactored from
`rust_provider.rs`'s raw bare-string extraction (round-4 R5: real new work):
- **method index** `BTreeMap<(TypeKey, name), Vec<MethodEntry>>`, `MethodEntry{fn_id, facts}`,
  `MethodFacts{kind: Inherent|Trait(trait: ScopeId), has_self, recv_mode, arity_excl_self, cfg}` — derived
  from the **AST node** (round-3 #8b). Keyed under the **defining type's `TypeKey`** (incl. `External(name)`
  for extension impls — round-4 F1) AND dual-keyed under the implemented trait's `TypeKey` (preserving today's
  dual-keying that powers `trait_dyn_dispatch`).
- **struct-field index** `BTreeMap<(TypeKey, field), Vec<(Option<CfgCond>, TypeKey)>>` — values resolved **in
  the struct's defining scope** (round-3 R9).
- **fn-return index** `BTreeMap<FnKey, Vec<(Option<CfgCond>, TypeKey)>>` — free fns (NEW) + impl methods;
  `Self`→owner; values resolved in the fn's defining scope; cycle-capped (§4.6).

**cfg-conditioning (round-3 R8, SOUND):** values are `Vec<(Option<CfgCond>, TypeKey)>`; with no call-site
`CfgCtx` (`engine.rs:484`), keep same-typed alternatives, drop on conflict, never select. Values are resolved
identities (or `External`); unresolvable → absent → fall through. Fail-open. (`rust_provider`'s nested-items
TODO → those receivers unrecovered → residue.)

### §3.2b The local-binding facts (build-time direct lookup; def-byte key — round-3 R2, round-4 F5)
`BindingRef{scope, ordinal}` is non-unique (`walk/locals.rs:286`). The build-time typer therefore performs a
**direct visible-binding lookup** (the rib search returning the `Binding`) for the receiver name at the call
site and reads its `Span` (→ `def_byte`); it does **not** `resolve`-then-read (which loses the binding —
round-4 F5). `local_facts: BTreeMap<(FileId, def_byte), LocalFact>`,
`LocalFact{kind: Param|Let|Pattern, annotation: Option<TypeName>, init: Option<InitExpr>, place}`, built+
consumed at build time. (Phase-3: harden `BindingRef`, fold `ty` onto `Binding`; §11.)

### §3.3 The build-time ladder + the read split (PRESERVES today's flow)
**Build time:** for each `x.m(args)`, run `RustReceiverTyper`. `None` → leave the site (→ today's residue path
unchanged). `Some(chain)` → materialize `T = chain[0].key` (+ `via`) onto the `CallSite`.
**Read time (`resolve_call_site`):** if a `T` is materialized → `cands = method_index[(T, m)]`
(`has_self`-filtered, keyed by the resolved identity if both sides resolved, else the bare-string fallback) →
`arity_filter` → **the kind-aware combine (round-5 G1): inherent-single → Exact; trait-impl/wrapper-peeled-
single → NameOnly-demote; multi → TraitCha-demote; empty → drop** (today's `:788`/`:833` paths, now over the
identity index and split by `MethodFacts.kind`). No materialized `T` → the existing residue (only on
unrecovered, `:837`). This **preserves edge presence** (every fixture caller-set intact — NameOnly edges are
included), the wrapper peel, and the drop-on-miss / residue split; it **refines confidence** (trait/wrapper-
single Exact→NameOnly — P/R-neutral, honesty-positive) and changes *which* receivers are recovered (more) and
*how owners are keyed* (qualified-when-both-resolve, else bare).

---

## §4 Rust `RustReceiverTyper` + indices (specified in full)

**Recovery (build-time; in order; first certain wins; shadow/poison/generic/inference → `None`):**
1. `self`/`Self` → `method_owners[caller]` resolved via §3.1. `self.f.m()` → field index (`FieldTyped`).
2. receiver → **param** `x: T` → `T` (`TypedParam`); `x: &dyn Tr` → trait `TypeKey`.
3. …**typed let** `let x: T` → `T` (`TypedLet`).
4. …**constructor let** `let x = T::new()`/`T{…}` → `T` (`ConstructorLocal`), binding-disambiguated.
5. …**field access** `let x = e.f` / `self.f` → resolve `typeof(e)` (recurse, capped §4.6) → field index
   (`FieldTyped`).
6. …**call** `let x = g(…)` / `g()` → resolve `g` → fn-return index (`ReturnTyped`). **Cycle/depth cap:** depth
   ≤4; a revisited `((file,def_byte) | FnKey)` → `None`.
7. **std-wrapper** `Box/Arc/Rc/Pin<T>` → peel to `T` (`StdWrapperPeel`) — **preserved as today**.
8. else (generic-param, closure, inference, unresolvable) → `None`.

**Index build (§3.2):** the identity-aware extractor populates the CallGraph-resident method/field/return
indices with resolved `TypeKey`s (incl. `External` for extension impls), AST-`MethodFacts`, `Self`/alias
resolution, cfg-conditioning; free-fn returns are NEW. Built whole-repo alongside `populate_scope_graph`,
serialized, cache-versioned.

**Routing (§3.3):** the kind-aware `owner_lookup` combine over the identity index (with bare fallback when
identity is unresolved) — **inherent-single → Exact**; **trait-impl/dyn-single/extension/wrapper-peeled-single
→ NameOnly-demote**; **multi → TraitCha-demote**; **empty → drop** — plus `has_self`, arity; unrecovered →
residue.

**Phase-3 (model-represented; 2a preserves/defers):** inherent-vs-trait soundness (inherent→Exact,
trait→demote unless trait-in-scope + unshadowed) + applicability (`ReceiverPlace`); trait-object/generic-bound
dispatch (`rust_provider.satisfaction`); wrapper/`Deref`-aware dispatch (the `Arc::clone` precision fix);
method chains; binding-types-on-the-graph (§11); Go field/return gaps; Python/TS typers.

---

## §5 Pressure test: does the model fit the other languages? (owner-requested)

- **Go (proven precedent — FOLDED-SOUND).** Concrete receivers → `owner_lookup`; interface receivers →
  `interface_impls` as a **fallback rung after an owner_lookup miss** (`resolution.rs:788-833`); Go's recovery
  variants (`:203-214`) are the Go partition of `ReceiverRecovery`. The build-time receiver materialization
  (`receiver_type` on `CallSite`) that rev 5 extends is the shipped Go mechanism. **No Go change.**
- **C++.** Type from declared type / `auto x = make()` / `obj.field`; the `type_db` (clang) is C++'s type
  source where Rust uses the extracted indices. Overload sets → `ResolvedSet`. **Fits.**
- **Python.** `x.method()` from `__init__`/annotations/`x = Foo()`; inheritance/MRO is the trait-dispatch
  analog (deferred). **Fits.**
- **TS/JS.** TS annotations → fits; untyped JS → mostly fall through. **Fits.**

**Summary:** the language-neutral generalization of the **already-shipped Go** receiver dispatch (build-time
materialization + fallback interface rung + a language-partitioned recovery enum). Rust is the next
instantiation over CallGraph-resident, identity-keyed indices; C++ uses the type_db; Python/TS later.

---

## §6 Form → resolution mapping (coverage)
| Rust receiver form | Type source | Phase-2a resolution | Phase |
|---|---|---|---|
| `self`/typed-param/typed-let/constructor, **inherent** single `m` | annotation / `local_facts` | **Exact** | 2a ✦ |
| `let x = obj.f; x.m()` **inherent** single `m` | **field index (new)** | **Exact** (recall add) | 2a ✦ |
| `let x = g(); x.m()` **inherent** single `m` | **fn-return index (new)** | **Exact** (recall add) | 2a ✦ |
| `f: Fast; f.go()` (single **trait** impl) | param | **NameOnly** (caller-set preserved — `trait_static_dispatch`) | 2a ✦ / 3 (→Exact) |
| `r: &dyn Runner; r.go()` (single impl) | trait dual-key | **NameOnly** (caller-set preserved — `trait_dyn_dispatch`) | 2a ✦ / 3 (→Exact) |
| `Box/Arc/Rc/Pin<T>` receiver, single `m` | **peel to `T`** | `owner_lookup` → **NameOnly** (wrapper-peeled) | 2a ✦ / 3 (→Exact) |
| extension impl on external `T` (`my_str.ext()`), single `m` | canonical `External(T)`-keyed index | **NameOnly** (trait) / **Exact** (inherent-style) | 2a ✦ |
| multi-impl / `dyn` >1 / name on >1 owner | identity index | **TraitCha demote** (preserved) | 2a ✦ / 3 |
| recovered receiver, no in-repo `m` (external or in-repo) | empty lookup | **drop** (preserved; FPs die via recovery) | 2a ✦ |
| associated fn `x.new()` | `has_self=false` | **excluded** | 2a ✦ |
| cross-module `a::Foo`/`b::Foo` (both resolve) | resolved `ScopeId` | **no collision** (precision; never-worse) | 2a ✦ |
| receiver/impl type unresolved (populator gap) | — | **bare-string fallback** (today's behavior) | 2a ✦ |
| generic-param / closure / inference | — | **fall through → residue** (preserved) | 2a → 3 |
| unrecovered receiver | — | **residue → removed on §2.3 gate** | 2a/2b ✦ |
| raise trait/wrapper NameOnly→Exact; applicability / trait-in-scope; Deref; chains | place / NS_TYPE / satisfaction / return idx | (NameOnly / residue in 2a) | 3 |

---

## §7 Invariants (recall-safety is structural)
- **Non-removing by construction (the two safe levers):** *confidence refinement* (trait/wrapper-single
  Exact→NameOnly — P/R-neutral, every fixture caller-set intact since NameOnly edges are included) and
  *identity-or-fall-through-to-bare* (G2 — the resolved `TypeKey` keys the lookup only when BOTH impl-header and
  receiver resolve confidently, else today's bare-string `owner_lookup`; `External(name)` minted only for
  confidently-external, never for "unresolved"; extension impls keyed under canonical `External`, never
  pre-dropped) **remove no current edge**.
- **Sound-but-edge-removing (the measured lever — round-6 finding 1):** the **drop-on-miss for a
  newly-recovered receiver** (certainly typed, no in-repo `m`) *does* remove today's residue over-claim — this
  is where the receiver-method FPs die. It is sound (given certain recovery the residue edge was wrong), but
  per the owner's standard it is **edge-removing and therefore Tier-A-measured** as part of the 2a gate
  (precision up, recall held) — NOT asserted as free.
- **Kind-aware confidence (round-5 G1):** **inherent**-single → Exact (sound — inherent has priority);
  **trait/wrapper**-single → NameOnly (honest — in-scope/shadow unproven until Phase 3). Never a wrong Exact on
  a newly-recovered receiver.
- **Gated subtractions:** the 2a drop-on-miss (measured in the 2a gate), residue removal/tightening for
  *unrecovered* receivers (Phase 2b), and raising trait/wrapper-single NameOnly→Exact + wrapper/Deref precision
  (Phase 3) are all Tier-A-gated.
- **Build-time/read split:** the typer (needs AST) runs at build time and materializes onto the `CallSite`;
  `resolve_call_site` is a pure read.
- **cfg:** keep same-typed alternatives, drop on conflict, never select.
- **Determinism:** `BTreeMap` indices; resolved `ScopeId`/def-byte keys (rebuilt together — §8);
  cfg-conditioned (never merged).
- **Tier-A gate:** Phase 2 shows prism `callers`/`C-method` **precision up** with a **positive golden set**
  (correct receiver-typed edges asserted exactly) and **recall reported** (a dip blocks any gated subtraction).

## §8 Incremental & cache
The identity-keyed method/field/return indices + `local_facts` are built on `CallGraph` in the whole-repo
build pass (alongside `populate_scope_graph`), the receiver `TypeKey` materialized onto each `CallSite`, all
serialized, **`CACHE_VERSION`-bumped**. **`ScopeId` is not content-stable across rebuilds** (`types.rs:24-29`),
so the scope graph, the indices, and the materialized `TypeKey`s are **rebuilt together**; incremental rebuild
must **re-materialize all receiver outcomes**, not merge stale call sites (round-4 F6; today's incremental path
`cpg/build.rs:222-234` merges stale sites — the plan must change this for the receiver outcomes). Fail-open: a
malformed def → that entry absent → fall through, never a panic.

## §9 Phasing
- **Phase 2a (additive recall + best-effort identity + a measured drop-on-miss precision lever):** the build-time
  `RustReceiverTyper` (incl. the preserved wrapper peel); **resolve-both-or-bare identity** (the
  `resolve_type_path_to_type_scope` pass + the direct binding lookup) keying an identity-aware CallGraph-
  resident method index (incl. canonical `External`-keyed extension impls), falling back to bare strings when
  unresolved; the **kind-aware combine** (inherent-single → Exact; trait/wrapper-single → NameOnly;
  multi → demote; empty → drop); **field + fn-return recovery**; `has_self` + arity; the
  materialize-onto-`CallSite` + read split. **The wrapper peel and the recovered→drop-on-miss / unrecovered→
  residue split are structurally unchanged; confidence refinement + identity-fallback remove no edge.** The
  one edge-removing lever is the **drop-on-miss for newly-recovered receivers** — sound (the removed residue
  over-claim was wrong) but **Tier-A-measured** in the 2a gate: the receiver-method FPs die there, the
  cross-module FPs via best-effort identity. The 2a PR reports precision up + recall up/held.
- **Phase 2b (telemetry + gated subtractions):** `ReceiverRecovery`/`MethodFacts` histogram via `nav
  call-stats`; the precision-floor report; **residue removal/tightening** — only after Tier-A (the §2.3 gate).
- **Phase 3 (precision the model represents):** **raise trait/wrapper-single NameOnly → Exact** via
  inherent-vs-trait soundness + applicability (`ReceiverPlace`) + trait-in-scope (NS_TYPE resolution of the
  trait from the call scope); trait-object/generic-bound dispatch over `satisfaction`; wrapper/`Deref`-aware
  dispatch (the `Arc::clone` fix); method chains; binding-types-on-the-graph (§11); Go field/return +
  cross-package gaps; Python/TS typers.

## §10 Consumers
- **`resolution.rs` R6** (`resolve_call_site`, ~:781): reads the build-materialized receiver `TypeKey`; runs
  the **kind-aware** combine (inherent-single→Exact / trait-wrapper-single→NameOnly / multi→demote / empty→drop) over the identity index (bare fallback when unresolved) + arity. The
  name-based residue is **unchanged** (unrecovered-only) until the Phase-2b gate.
- **`CallGraph` build:** runs the build-time `RustReceiverTyper` + the identity-aware extractor; materializes
  the receiver `TypeKey` onto each `CallSite` and the indices onto `CallGraph` (rebuilt-together — §8).
- **`nav callers`/`callees`/`call-stats`:** receiver-typed edges flow through the existing call graph;
  `call-stats` gains the recovery histogram.
- Non-Rust receiver dispatch (Go) is **unchanged**.

## §11 Relationship to the Phase-1 scope graph
Phase 2 **reads** the scope graph (build time) for the receiver's binding (direct lookup → `Span`/`def_byte`),
the `resolve_type_path_to_type_scope` identity resolution, and (Phase 3) trait-in-scope NS_TYPE resolution; it
**adds** the type layer beside it (the def-byte `local_facts` + the CallGraph-resident identity indices). It
does **not** put types into the scope graph in Phase 2 — `Binding` carries no kind/type/AST node
(`types.rs:401-409`) and `BindingRef` is not yet unique (`types.rs:340`), so the build-time side-tables are the
surgical seam. **Phase-3 consolidation (optional):** harden `BindingRef` to a stable per-scope ordinal and fold
the recovered `ty` onto `Binding`. The Phase-1 `ResolveQuery`/`Target` shapes are unchanged; receiver-typing is
a consumer of name resolution + a build-time type layer, not a change to the graph.
