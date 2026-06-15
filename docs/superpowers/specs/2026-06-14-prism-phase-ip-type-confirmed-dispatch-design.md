# Phase-IP — Go Interface Dispatch — FOUNDATION (rev 5)

> **rev 5 (2026-06-15) — round-4 dual review folded; ready to plan.** A fourth dual review (codex
> rigor gpt-5.5 xhigh + claude soundness operator-subagent) ran on rev 4 and folded into rev 5:
> 4 BLOCKERs + 3 MAJORs + 3 MINORs, all **buildability/accuracy** — none touch the owner-locked
> decisions. Key folds: the `I-dispatch` **stratum** was infeasible (strata are seed-time) → pivoted to
> per-site `dispatch_kind` metadata (§14); generics can't be gapped by a context-free `canon_type` →
> gated at the `type_parameter_list` declaration (§6); §17 CHA claim was **factually wrong** (the C++
> override index is name-keyed over *all* functions) → filter seed+candidates to `type_db`-owned (§17);
> §13.1's "fallback-Exact is load-bearing for the flip" was **false** (the matrix is confidence-blind)
> → corrected + a fallback-under-ExactOnly fixture added (§13); gap taxonomy split fatal-vs-admitted
> (§15); `CACHE_VERSION` bump to **9** (§11); scoped-build reworded (subset, nav never scoped). Records:
> `docs/prism-query-layer/phase-ip-spec-review4-{codex,claude}-2026-06-15.md`.

> **rev 4 (2026-06-15) — the FOUNDATION increment.** After three review rounds the
> interface half was kept back; **embedding shipped** (`2026-06-15-prism-phase-ip-go-embedding-design.md`,
> merged `e03f547`). Owner decision 2026-06-15 (**Option 2**): split the interface work in two —
> **(PR-1, this spec)** the dispatch *engine* + the harness *attribution apparatus* for the receivers
> P6-lite **already types** (typed params + constructor locals); **(PR-2, deferred)** expand P6-lite to
> recover **type-assertion / `var` / interface-slice** receivers, where caddy's *corpus* recall actually
> lives. PR-1 flips the in-repo `go/interface_dispatch` capability (its fixture is a typed-param
> receiver) and builds the measurement so PR-2's gate delta is cleanly attributable; PR-1 is **caddy
> corpus-neutral by construction**. The **PR-2 work-list is carried at the end** of this spec.

**Date:** 2026-06-14 (rev 5: 2026-06-15) · **Status:** rev 5 — foundation, awaiting user review → writing-plans.
**Builds on:** **embedding** (`e03f547`), **EFT** (`e7a37e5`), **pre-IP** (`710cb86`). Closes the
typed-param-receiver Go half of `s3-deferred §1`/`§2`.
**Review lineage (folded):** round 1 (codex rigor 4 BLOCKERs + claude "sound-to-plan"), round 2
(codex+claude on the Option-B §6/§7 surface), round 3 (codex+claude xhigh, both "needs changes" — the
interface half), **round 4 (codex rigor xhigh + claude soundness on rev 4 — both "needs changes",
4 BLOCKERs + 3 MAJORs + 3 MINORs, all buildability/accuracy)**. Records:
`docs/prism-query-layer/phase-ip-spec-review{,2,3,4}-{codex,claude}-2026-06-1{4,5}.md`. rev 4 folded the
round-3 work-list + re-anchored every citation to post-`e03f547`; **rev 5 folds round 4** (see the
rev-5 banner) and is verified buildable against the post-`e03f547` tree (the §17 CHA dispute was
adjudicated against `build.rs:534` — codex correct, claude's "sound" overturned).

**Owner decisions (locked):** (1) *earn the Exact* — interface dispatch is `Exact` via
**signature-confirmed** satisfaction; (2) **keep the empty-intersection fallback Exact**, make liveness
comprehensive (within what is statically scannable) so it rarely fires; (3) **uncapped** fan-out —
genuinely-live wide fan-outs enter `ExactOnly`, guarded by a precision gate + telemetry, not a width
cap; (4) **Option 2 split** — typed-param receivers + engine + attribution now (PR-1), receiver
expansion later (PR-2).

---

## 0. Scope of this increment (PR-1)

**In scope.** Named, in-repo Go **interface dispatch** for receivers **P6-lite already types** —
typed params (`func run(r Runner)`) and constructor locals — resolved **Exact / `InterfaceDispatch`**
via **signature-confirmed** structural satisfaction, **receiver-kind-aware** (Go's `*T`-set ⊇ `T`-set),
**RTA-pruned** to live implementers, **multi-implementer** (N callees). Plus the **harness attribution
apparatus** (a resolution-kind on each SUT call edge, a fingerprinted interface-site manifest, an
interface stratum) so interface precision is measurable.

**Out of scope here (→ PR-2 work-list at end).** Expanding P6-lite to recover **type-assertion**
(`x.(Module).CaddyModule()` — the "57 caddy sites"), **`var r Runner`** locals, and **interface-slice**
element receivers. These are where caddy's *corpus* recall lives; the engine PR-1 builds resolves them
the moment PR-2 feeds it their receiver types — no engine rework.

**Out of scope entirely (→ §18).** Python; `from_import_alias`; Rust S3.1; Go generics / type-sets;
anonymous interfaces; non-local-construction liveness; precise cross-package keys; the fan-out width-lever.

**What moves.** The in-repo `go/interface_dispatch` capability flips `known_fail → pass` (its fixture
is `func run(r Runner){ r.Go() }`, already P6-lite-typed). The **caddy corpus metric is expected
~neutral** (its recall is in type-assertion sites PR-2 owns) — PR-1's value is the capability flip, the
engine, and the measurement.

## 1. The decisive constraint (unchanged)

Success signal = **caddy nav recall** (prism `nav callers/callees` vs gopls). Nav and CPG resolve
through the **same ladder** `resolve_call_site`; nav re-resolves on `cg.call_graph` and **never reads
CPG edges** (EFT §0; re-verified rev 4 — `direct_callers` queries.rs:247-262, `direct_callees`
queries.rs:362-389). Dispatch **must flow through `resolve_call_site`**. A build-time CPG-only post-pass
is **rejected** (would not move nav recall).

## 2. Architecture — extend the already-consumed GoTypeProvider

Build order (re-verified): `CallGraph::build` (call_graph.rs:219) runs **before** the `TypeRegistry`
(context.rs); `resolve_call_site` materializes edges in `assemble_graph` Step 5 (cpg/build.rs:352-382)
and is the nav query path → it cannot reach the registry. **Embedding already established the pattern:**
`CallGraph::build` ends by calling `apply_go_embedding_promotion(files)` (call_graph.rs:717), which
constructs `GoTypeProvider::from_parsed_files(files)` (go.rs:111) and writes CallGraph-owned,
serialized maps (`promoted_aliases`, `embedding_gaps`, call_graph.rs:71-78). Interface dispatch adds a
**sibling** apply that writes one more CallGraph-owned map (`interface_impls`), built from the **same
provider** via a new public method. No new `src/dispatch/` module; the registry's registered provider
stays the future algorithm-facing API.

| # | Approach | Verdict |
|---|---|---|
| **1** | Consume `GoTypeProvider` in `CallGraph::build` (sibling to embedding); `resolve_call_site` reads precomputed `interface_impls`. | **CHOSEN** — only path through the one ladder; reuses the provider + the embedding apply/clear pattern; registry-independent. |
| 2 | Build-time CPG post-pass (Step-9 CHA mirror). | Rejected — nav never reads CPG edges → metric unmoved. |
| 3 | Thread `TypeRegistry` into `resolve_call_site`. | Rejected — registry absent at Step-5; invasive. |

## 3. Decisions (owner-locked; rev 4)

| Decision | Choice |
|---|---|
| Scope | Go **named in-repo** interface dispatch for **P6-lite-typed receivers** (typed params + constructor locals). Type-assertion/`var`/slice receivers → PR-2. Anonymous interfaces, generics/type-sets, Python, Rust S3.1 → §18. |
| Confidence | **Exact, earned** — signature-confirmed (§6), receiver-kind-aware (§7), RTA-confirmed (§8). Kind `InterfaceDispatch`. |
| RTA fallback | **Keep Exact** (owner). `scan_go` comprehensive over statically-scannable construction (§8); fallback is **receiver-kind-aware-empty** (fires only when **no** admission key of any kind is live for `I`). |
| Fan-out | **Uncapped** — live wide fan-outs enter `ExactOnly` as sound may-targets; guarded by the §14 gate + fan-out telemetry, **not** a width cap. Width-lever deferred (§18). |
| Satisfaction | **Signature-confirmed** (canonical sig §6); upgrade `compute_satisfaction` from name-only (go.rs:464-473). |
| Receiver kind | **Three keys** (§7), separate contracts (§10, BLOCKER-2): admission `T`/`*T`, method-body bare `T`, interface key. |
| Keys | **No single `normalize_go_key`.** Four explicit contracts (§10): `owner_key` (existing, non-Go + bare owner) · `iface_key` (interface lookup: strip `pkg.`, gap on `[…]`) · `admission_key` (pointer-preserving) · gap-key (unsupported → recorded gap). |
| Provider API | **New public methods on `GoTypeProvider`** (§10, BLOCKER-3) — `GoTypeData`'s maps stay private; expose a single `compute_interface_dispatch(live) -> InterfaceDispatchTable`. |
| Resolution kinds | Add `InterfaceDispatch` → `Exact` via `exact()`; `as_str()` arm `"interface_dispatch"` (kind rides into `Reason::Resolution`, already emitted — §14). |
| Cache | **Bump `CACHE_VERSION` 8→9** (cpg_cache.rs:49). New `interface_impls`/`interface_gaps` fields; `#[serde(default)]` is for JSON, but bincode does **not** supply it for new trailing fields (cpg_cache.rs:49 comment) — a bump is the explicit, safe choice, consistent with embedding's 7→8 precedent (round-4 codex MINOR). |
| Build paths | **Replace-not-merge** on all three paths (§11), via a sibling `clear_interface_dispatch` hooked into `remove_files` (mirrors `clear_promoted_embedding`). |
| Scoped build | Dispatch computed over the **scoped subset** (same as embedding) — **not** full-repo; only affects best-effort slice edges. Nav (the metric) **never uses `build_scoped`** (`mod.rs:34` debug-asserts whole-repo), so it always resolves on the full-repo owner index + `interface_impls` (§11, round-4 codex MAJOR / claude MINOR-1). |
| Gap taxonomy | **Two categories** (§15, round-4 codex MAJOR): **fatal `GoDispatchGap`** (generics, anonymous interface, unknown `canon_type`) mints **no edge**; **admitted `GoDispatchOverApprox`** (cross-pkg bare-name, non-local-construction fallback) **mints the Exact edge** + a telemetry counter. "Recorded, never matched" applies only to the fatal set. |
| DataFlow fan-out | Step-5b arg→param edges fan out per live satisfier (sound may-flow); **telemetered**, tested by §13.8 (§16). |

## 4. Mechanism A — Go embedding (SHIPPED `e03f547`)

Embedding (owner-index promotion, receiver-kind-aware) is **merged** and is **not respecified here**.
Interface dispatch *builds on* it: `interface_impls` is a sibling field to `promoted_aliases`
(call_graph.rs:71-78); the interface apply is a sibling to `apply_go_embedding_promotion`; the interface
clear is a sibling to `clear_promoted_embedding`. The two mechanisms are independent at resolution time
(embedding hits via `owner_lookup`; interface fires on its **miss**, §5).

## 5. Mechanism B — Go interface dispatch (signature-confirmed multi-implementer)

New CallGraph field (call_graph.rs, beside `promoted_aliases`):
```rust
#[serde(default)]
pub interface_impls: BTreeMap<(String /*iface_key*/, String /*method*/), Vec<FunctionId>>,
#[serde(default)]
pub interface_gaps: BTreeMap<String /*GoDispatchGap kind*/, usize>,
```
Init `BTreeMap::new()` in all four constructors (`empty`, `build_skeleton`, `build`, `build_direct_subset`).

Precompute in `CallGraph::build` (a sibling call right after `apply_go_embedding_promotion(files)`,
call_graph.rs:717):
```rust
self.apply_go_interface_dispatch(files);   // NEW sibling
```
`apply_go_interface_dispatch(&mut self, files)`:
1. `self.clear_interface_dispatch();` (idempotent replace, §11).
2. Early-return if no Go files present (mirror embedding, call_graph.rs:816-820).
3. `let live = go_admission_live_set(files);` — admission-keyed live set (§8).
4. `let provider = GoTypeProvider::from_parsed_files(files);`
5. `let table = provider.compute_interface_dispatch(&live);` (§10, the sole public entry).
6. For each `((iface_key, method), fids)` in `table.impls` **with `!fids.is_empty()`**: store into
   `interface_impls`. (Empty → not stored; the seam then drops explicitly — codex r1-#2.)
7. Fold `table.gaps` into `interface_gaps`; fold `table.fanout` into telemetry (§10).

**Resolution seam** (resolution.rs — the R6 P6-lite branch; post-merge the `owner_lookup` miss →
`ExternalReceiver` drop is at resolution.rs:438). Replace the terminal `None ⇒ ExternalReceiver` arm:
```rust
// recv_ty recovered by P6-lite (TypedParam / ConstructorLocal), name = method
match self.owner_lookup(recv_ty, name) {
    Some(callees) => /* existing: includes embedding's EmbeddedPromotion relabel */,
    None => match self.interface_impls.get(&(iface_key(recv_ty), name.to_string())) {
        Some(ids) if !ids.is_empty() =>
            ResolutionOutcome::hit(exact(ids.iter().copied().collect(),
                                          ResolutionKind::InterfaceDispatch)),
        _ => ResolutionOutcome::dropped(DropReason::ExternalReceiver),
    },
}
```
**`iface_key` is fallible** (round-4 codex): a generic/unsupported recovered receiver type yields no key
→ the `interface_impls` consult is **skipped** and the site drops `ExternalReceiver` (the lookup is never
attempted with a malformed key; the corresponding decl-side gap was already recorded at build, §15).

`exact(...)` accepts N callees (the R1 trait dual-key shape) and composes with Step-5 (one
`Call(Exact)` per callee, cpg/build.rs:373) and nav re-resolution. The seam is on the
**P6-lite-recovered branch only** — receivers P6-lite cannot type still drop upstream (PR-2 expands
*which* receivers P6-lite types; the engine here is unchanged by that). Step-9 CHA is `type_db`-gated
(C++) and cannot mint Go edges (§17).

## 6. Canonical signature — `canon_type` / `canon_sig` (rev 3 core + rev-4 completeness)

Today `extract_method_signature` (go.rs:365) and `extract_func_signature` (go.rs:445) emit **raw text
with param names**, and `compute_satisfaction` (go.rs:464) is **name-only** (keys-only check,
go.rs:471-473). rev 4 defines **one `canon_type(node) -> CanonResult`** and **one
`canon_sig(params_node, result_node) -> CanonResult`**, applied **identically** to interface
`method_spec`, concrete `method_declaration`, and flattened embedded-interface methods (canonicalize
**at extraction** so `collect_interface_methods_from` go.rs:495 merges already-canonical sigs; drop a
name-collision across embeds with unequal `canon_sig`). `CanonResult` is either a canonical `String`
**or** a `GoDispatchGap` (§15) — an unknown node never yields a string.

- **`canon_sig`** = `(` join(param `canon_type`, `,`) `)` `(` join(result `canon_type`, `,`) `)`.
  Names + blank `_` dropped; grouped params expanded (`a, b int`→`int,int`); **variadic** `...T` →
  `...`+`canon_type(T)`; **named multiple results** `(a int, b error)` → names dropped, types kept;
  **parenthesized single return** `(T)` ≡ unparenthesized `T` → one-element list; zero params/returns →
  empty list.
- **`canon_type`** (recursive, per Go type node):
  - `type_identifier` / `qualified_type` → **bare name** (`pkg.T`→`T`, via `iface_key`/bare-name rule).
  - `pointer_type` → `*` + inner; `slice_type` → `[]` + inner; `array_type` → `[N]` + inner.
  - `map_type` → `map[`K`]`V; `channel_type` → **direction-preserving** `chan`/`chan<-`/`<-chan` + inner.
  - `function_type` → `func(` params `)` results (recurse).
  - `interface_type`: empty `interface{}` / `any` → **`any`**; **non-empty anonymous interface** →
    `GoDispatchGap::AnonymousInterface` (the enclosing method is non-dispatchable, §15).
  - **Generic *instantiation*** (`generic_type`, `type_arguments` `[…]`) → `GoDispatchGap::Generic`.
  - **Any unenumerated node kind** → `GoDispatchGap::UnknownCanonType` (fail closed; never a silent
    false match).

**Generics are gated at the DECLARATION, not via `canon_type` (round-4 codex BLOCKER).** A *use* of a
type parameter (`func (r R[T]) M(x T)`) appears in the signature as a bare `type_identifier` `T`,
indistinguishable from a concrete type without the enclosing scope — a **context-free `canon_type(node)`
cannot recognize it** and would canonicalize `T` to a bare name and falsely match. So **any interface or
method whose declaration carries a `type_parameter_list`** (or an interface carrying a type-set
`type_elem`) is marked **non-dispatchable (`GoDispatchGap::Generic`) at extraction**, before `canon_type`
runs — detected from the declaration node, not threaded as scope through the recursion. `canon_type` stays
context-free; it never gaps a bare type-parameter use (the declaration gate already removed it) but still
gaps `generic_type`/`[…]` instantiations it directly sees. This fails closed without a type-parameter scope.

- Satisfaction (`compute_satisfaction`): `T` satisfies `I` iff for every `m ∈ M_I` there is a method
  named `m` on `T`'s (§7) method set with **equal `canon_sig`** (both sides gap-free).

**Cross-package over-approx (codex r2-m2):** bare-naming means `Read(io.Reader)` and `Read(bufio.Reader)`
canonicalize equal — a measured over-approx (precise cross-pkg keys → §18; measured by §14).

## 7. Receiver-kind correctness — three keys (rev 3 carry-forward)

Go method sets are asymmetric (`*T` set ⊇ `T` set). `GoMethod.is_pointer_receiver` is tracked (go.rs).
Three producers share one invariant:

- **Admission key** (satisfaction map values + live set): value-receiver satisfier contributes `T`;
  pointer-receiver-only satisfier contributes `*T`.
- **Method-body key** (`data.methods`, FunctionId lookup): bare `T` — the target FunctionId is
  identical for `T`/`*T`.
- **Interface key**: `iface_key(I)`.
- Satisfaction: `T` satisfies via `set_value(T)` = `{m : !is_pointer_receiver}` (+ value-promoted);
  `*T` satisfies via `set_ptr(T)` = all (+ promoted). The satisfying admission key is recorded.
- `resolve_dispatch` intersects admission keys against the §8 live set; fallback per §5.

**Invariant (so a partial impl can't silently break the intersection):** the satisfaction map, the
live set, and the intersection all use the admission-key alphabet `{T, *T}`; the method-body lookup
uses bare `T` after admission.

## 8. Liveness — comprehensive within what is statically scannable (rev-4 honest fix)

**Citation correction.** rev 3 §8 cited `ast.rs:3899` (`constructor_type`) for a "factory-return"
rule. That function recognizes constructor **call/literal sites** (`Ty::new()`, `NewTy()`, `Ty{}`), **not
function return types** — it cannot drive factory-return liveness as written (codex r3 / claude r3-A3,
confirmed rev 4). The honest, *stronger* rule: **local construction is caught wherever it appears —
including inside factory bodies** — so a factory `func New() *Foo { return &Foo{} }` is covered by
scanning its body's `&Foo{}`. The residual gap is only **non-local construction** (reflection,
cross-module-only instantiation, returned-from-callee values), recorded as a gap and covered by the
kept-Exact fallback.

`scan_go` **already recurses the whole AST** via `scan_tree_recursive` (live_types.rs:275) — factory and
all function bodies are **already visited today** (round-4 claude MINOR-2; this is why local construction
inside factory bodies is already caught). The change is **only** to the per-node handler `scan_go_node`
(live_types.rs:158, today `composite_literal`-only, strips `&`) to emit the **admission-key alphabet**
`{T, *T}` — not to add traversal scope:
- `T{}` (composite literal, value) → `T`.
- `&T{}` (composite literal under `unary_expression`/`&`) and `new(T)` (call to builtin `new`, first
  type arg) → `*T` **and** `T` (both method sets become live; `&T{}` is addressable).
- `var x T` (`var_declaration` → `var_spec` with a concrete `type` field) → `T`.
- Qualified/pointer normalization via the bare-name rule; pointer-ness preserved in the admission key.
- **Disambiguation (round-4 codex):** §8's `var x T` is a *liveness* signal where **`T` is a concrete
  type** (marks `T` live). It is distinct from PR-2's `var r Runner`, where **`Runner` is an interface**
  used as a *receiver* (receiver recovery, deferred). `scan_go_node` marks only concrete `var x T`; it
  never types an interface-typed local as a receiver (that is PR-2's job).

`go_admission_live_set(files)` runs this Go-scoped scan and returns `BTreeSet<String>` over admission
keys (`"T"`, `"*T"`). **Other languages' `scan_*` and the C++ CHA live set (`collect_live_classes`,
cpg/build.rs:531) are untouched** (§17). Residual gap → `GoDispatchGap::NonLocalConstruction` (§15);
the §5 receiver-kind-aware fallback is the safety net (kept Exact per owner; over-approx measured §14).

## 9. Confidence + EFT reconciliation (unchanged)

Interface dispatch mints **Exact**, earned (signature-confirmed §6 + receiver-kind-aware §7 +
RTA-confirmed §8). The fan-out to N **live** satisfiers is N provably-possible targets — sound
may-analysis (same class as Step-9 CHA, no shared code path, §17). Interface edges enter `ExactOnly`
slices (the recall win lands there). **Accepted risk (owner):** a wide live interface (`error`) yields
a large *correct* Exact fan-out into precision-biased slices; rev 4 does **not** cap it — bounded by
RTA precision (§8), measured by the §14 gate + fan-out telemetry, with the §18 width-lever as escape
hatch if the gate trips.

## 10. Wiring, key contracts, and the provider API

**Key contracts (BLOCKER-2 — separate, not one normalizer).** rev 3's single `normalize_go_key` is
**rejected**: one function cannot both strip pointers and preserve the `T`/`*T` admission key §7 needs,
nor collapse generics §6 must gap. Four explicit contracts:

| Contract | Strips / rule | Used for |
|---|---|---|
| `owner_key` (existing, resolution.rs:74-86) | `&`,`*`,`dyn`/`impl`,`<…>`, `::`-namespace tail. **Not** `pkg.`/`[…]`. | non-Go owners + bare-owner keys (unchanged). |
| `iface_key(s)` (new) | strip `pkg.` (qualified→bare), no pointer; **`[…]`/generic → gap** (not a key). | interface lookup at the seam + `interface_impls` keys. |
| `admission_key` (new, typed) | pointer-**preserving** `T` vs `*T`; bare part via `iface_key` rule. | satisfaction values + live set + intersection alphabet. |
| gap-key | unsupported `canon_type`/iface → `GoDispatchGap` record. | telemetry only, never matched. |

**Provider API (BLOCKER-3 — `GoTypeData` maps stay private).** Add to `GoTypeProvider` one public entry
plus thin accessors:
```rust
pub struct InterfaceDispatchTable {
    pub impls: BTreeMap<(String /*iface_key*/, String /*method*/), Vec<FunctionId>>,
    pub gaps:       Vec<GoDispatchGap>,        // FATAL — non-dispatchable, no edge (§15)
    pub overapprox: Vec<GoDispatchOverApprox>, // ADMITTED — edge minted, telemetry (§15)
    pub fanout: BTreeMap<(String, String), usize>, // per (iface,method) live width
    pub fallback_fired: BTreeMap<(String, String), bool>, // entry came from empty-live fallback (§13/§14)
}
pub fn compute_interface_dispatch(&self, live: &BTreeSet<String>) -> InterfaceDispatchTable;
```
`compute_interface_dispatch` keeps satisfaction/canon/admission logic **inside** go.rs (where the
private maps live), upgrading `compute_satisfaction` to signature-confirmed (§6) and receiver-kind sets
(§7), and applying RTA + the receiver-kind-aware fallback (§5/§8). It iterates **named in-repo**
interfaces only; for each method it records either satisfier `FunctionId`s (non-empty) or a gap.

**Wiring table (post-`e03f547` line numbers):**

| Item | Location | Change |
|---|---|---|
| `interface_impls`, `interface_gaps` | call_graph.rs:71-78 (beside `promoted_aliases`) | new `#[serde(default)]`; init in all 4 constructors (93-94 / 213-214 / 714-715 / build_direct_subset). |
| `apply_go_interface_dispatch` + `clear_interface_dispatch` | call_graph.rs (siblings of 813-865 / 798-809) | construct provider, `compute_interface_dispatch`, store; clear mirrors `clear_promoted_embedding`. |
| Sibling apply call | call_graph.rs:717 (after embedding apply) | `self.apply_go_interface_dispatch(files);` |
| `remove_files` clear hook | call_graph.rs:768 (next to `clear_promoted_embedding`) | also `self.clear_interface_dispatch();` |
| `canon_type`/`canon_sig` + satisfaction | go.rs (rewrite extractors 365/445; upgrade `compute_satisfaction` 464) | §6 grammar; receiver-kind sets §7; gap-returning. |
| `compute_interface_dispatch` (pub) | go.rs (new) | the sole public dispatch entry (above). |
| `go_admission_live_set` / `scan_go` | live_types.rs:154-172 | §8 admission-key alphabet. |
| `iface_key` / `admission_key` | resolution.rs (new) | §10 contracts (owner_key unchanged). |
| Seam consult + drop | resolution.rs:438 | §5 (interface consult on `owner_lookup` miss; explicit empty→drop). |
| `ResolutionKind::InterfaceDispatch` + `as_str` | resolution.rs:16-57 | new variant (beside EmbeddedPromotion:33/54) → `"interface_dispatch"`. |
| Telemetry | navigation/queries.rs (`call-stats`) | `interface_gaps` (fatal, by kind) + `interface_overapprox` (admitted, by kind) + `InterfaceDispatch` fan-out width + fallback-fired count (beside `embedding_gaps`). |

## 11. Cache & build paths

- **Bump `CACHE_VERSION` 8→9** (cpg_cache.rs:49; round-4 codex MINOR). The new `interface_impls`
  /`interface_gaps` fields are `#[serde(default)]`, but the code comment states **bincode does not
  supply `serde(default)` for new trailing fields** — an old cache would deserialize-miss (cpg_cache.rs:253,
  safe → rebuild), so "additive, no bump" is the wrong rationale. A bump is explicit and matches
  embedding's own 7→8 precedent for new CallGraph fields. GIT_SHA also invalidates the resolver change.
- **Replace-not-merge over all three build paths** (whole-program; codex r2-B6 / claude r2-B4):
  - `build` (call_graph.rs:219): provider over all `files`; sibling apply at 717.
  - `build_incremental` (after CG `merge`, which does **not** carry dispatch maps — call_graph.rs:775-791):
    call `apply_go_interface_dispatch(all merged files)` (clears + recomputes over the **full** file
    set) — closes the `remove_files`-by-`fid.file` stale-alias hazard exactly as embedding does.
  - `build_scoped` (context.rs:160-170 → `build_enriched(&filtered)` → `CallGraph::build(filtered)`,
    cpg/build.rs:134): Go dispatch is computed over the **scoped subset**, **same as embedding today** —
    **not** full-repo (round-4 codex MAJOR / claude MINOR-1; the rev-4 "full repo file set" claim was
    wrong, and it does **not** mirror `live_types`, which is a *separate* `collect_live_types(files)` over
    the full set, context.rs:170).
- **Scoped-build target existence (BLOCKER-4) — benign for a stronger reason.** Scoped dispatch over the
  subset may resolve to satisfiers absent from the scoped CPG node set → Step-5 simply **skips** those
  edges (cpg/build.rs:360-370): best-effort slice edges in scoped mode. Crucially, **nav never uses
  `build_scoped`** — `NavigationIndex::build` always calls whole-repo `CpgContext::build`
  (`mod.rs:34` `debug_assert!(ctx.scope.is_none())`). So the success metric (nav) always resolves on the
  **full-repo** owner index + `interface_impls`; the caddy metric is safe regardless of scoped behavior.
  **No new full-repo-dispatch-in-scoped build API is needed** (rejecting the codex "specify a new assembly
  path" fix — that work is unnecessary given nav is never scoped).

## 12. Failure modes

| Mode | Behavior |
|---|---|
| Interface, ≥1 live satisfier (admission key matches) | intersection → live satisfiers, Exact. |
| Interface, no admission key of any kind live | receiver-kind-aware fallback → full satisfier set, Exact (residual-gap net, §8; over-approx measured §14). |
| Interface, no in-repo satisfier | no `interface_impls` entry → explicit `ExternalReceiver` drop. |
| Value type, pointer-method interface | not admitted via `set_value`; admitted only if `*T` live (§7/§8). |
| Same name, different `canon_sig` | excluded (§6) — no false Exact. |
| Generic / type-set / anonymous-iface / unknown node | `GoDispatchGap` (§6/§15); never a false match, never a panic. |
| Non-local construction only | liveness gap (§8); fallback covers (Exact), over-approx measured §14. |
| Receiver P6-lite cannot type (type-assertion/`var`/slice) | drops upstream (out of scope; PR-2). |
| Warm cache pre-IP / incremental stale alias | GIT_SHA reject / replace-not-merge (§11). |
| Scoped build, target node absent | slice edge skipped; nav unaffected (§11). |
| Non-Go repos | maps empty; zero behavior change (regression guard §13.9). |

## 13. Testing & acceptance (pre-commit, no LSP)

1. **Interface basics:** (a) `r.Go()`→`Fast::Go`, `InterfaceDispatch`, Exact (the in-repo fixture,
   typed param). **Note (round-4 claude BLOCKER-1):** that fixture constructs nothing, so its RTA live
   set is empty — it resolves via the §5 **empty-live fallback**. But the capability **flip itself is
   confidence-blind**: the matrix runs default `nav callers` (no `--confidence`), emitting Exact *and*
   NameOnly and comparing **call-site locations only** (matrix.py:76; `exact=true` = set-equality, not a
   confidence assertion) — so it flips whether the fallback mints Exact *or* NameOnly. The fallback's
   *Exact-ness* matters only in **ExactOnly** traversal and is pinned **only** by (e)+§13.7 (do not
   assume the green matrix proves "keep fallback Exact"). (b) multi-implementer → both; (c) RTA pruning
   needs a *separate* fixture that **instantiates** `Fast` (and defines but never instantiates `Slow`):
   `live={Fast}` → only `Fast`; (d) receiver-kind-aware fallback (no kind live) → full set, Exact (a
   `resolve_call_site` unit assertion that the returned callees carry `ResolutionConfidence::Exact`);
   (e) **fallback survives ExactOnly (round-4 claude BLOCKER-1 — the linchpin test):** the empty-live
   fallback edge from (a) survives `nav callers --confidence exact` / the ExactOnly filter
   (queries.rs:296). If the fallback were demoted to NameOnly, (e) fails while (a) still passes — this is
   the only test that actually pins the owner's "keep fallback Exact" decision.
2. **`canon_sig` byte-equality (§6):** equal for param-name-only diff, grouped `(a,b int)` vs
   `(int,int)`, `interface{}`≡`any`, multi-return, single vs parenthesized return; **unequal** (→ no
   satisfy) for channel direction `chan<- T` vs `chan T`, return-type mismatch, arity mismatch;
   **variadic** `...T` and **named multiple results** canonicalize as specified.
3. **Gaps (§6/§15):** generic/type-set interface, non-empty anonymous-interface method, and an
   unknown/synthetic type node each → recorded `GoDispatchGap` (no false Exact, no panic); `call-stats`
   counter increments.
4. **Receiver kind (§7):** pointer-receiver-only type satisfies via `*T`; value `T{}`-only does not.
5. **Liveness (§8):** `new(T)`, `var x T`, `&T{}`→`*T`+`T`, and a factory body's `&T{}` are each live;
   a non-local-only construction is a recorded liveness gap (fallback path exercised).
6. **Capability flip (§14):** `eval/fixtures/go/interface_dispatch/expected.toml` `known_fail → pass`;
   update rationale; matrix asserts `ok`.
7. **Multi-implementer barrier precision (claude r1-MAJOR / r3-B8 / r4-BLOCKER-2 — gating fixture):**
   **TWO variants**, both seeded at an interface method with ≥2 implementers, asserting the `ExactOnly`
   barrier fan-out **and** the §16 DataFlow fan-out are **exactly** the intended set with **no
   non-satisfier leakage**: (i) **live-intersection** — implementers instantiated, `live={A,B}` → fan-out
   `{A,B}`; (ii) **empty-live fallback (round-4 claude BLOCKER-2)** — *nothing constructed* → the wide
   fallback mints the **full** satisfier set as Exact into the barrier slice; assert it equals the full
   satisfier set and that a non-satisfier bearing the same method name does **not** leak. Variant (ii) is
   the **only PR-1 guard on the uncapped-Exact decision** while the §14 corpus gate is dormant (§14e).
8. **Replace-not-merge (§11):** incremental rebuild after removing an implementer drops its interface
   edge (no phantom); after re-adding, it returns.
9. **Non-Go regression:** Rust/Python/C resolution byte-identical (maps empty).
10. **Cache:** new fields round-trip; cross-GIT_SHA rejected.
11. **Harness attribution (§14):** a unit test on `sut.py` parsing — an Evidence item whose `why[]`
    carries `Resolution{kind:"interface_dispatch"}` yields `CallEdge.resolution_kind ==
    "interface_dispatch"`; and the precision-gate site-filter selects that site by its **per-site
    `resolution_kind`/`dispatch_kind` metadata** (NOT a seed stratum — §14 was repivoted in round 4).

**Repo workflow:** `cargo fmt`; full `cargo test` (+`--features mcp`); `cargo build --release` then
`cd eval && uv run tier-a --matrix-only --allow-stale-sut` (interface gap `ok`, no other flips) then
`--quick`.

**Success metric (PR-1):** `go/interface_dispatch` flips `expected_gap → ok`; **no regression** — EFT
`target-c-method` exact P=R=1.0, default `flip_candidate`; prism/tokio/flask/click matrix + quick
unchanged. **caddy corpus is expected ~neutral** (recall is in PR-2's type-assertion sites) — this is
the intended outcome, recorded, not a miss.

## 14. Harness attribution apparatus + acceptance (BLOCKER-1)

rev 3's §14 gate was **vacuous** (the "57 caddy sites" are type-assertion → out of scope; no FP
attribution existed). rev 4 builds the apparatus so interface precision is *measurable*, scoped to
**in-scope** (P6-lite-typed) interface sites.

**(a) Surface the kind on every SUT call edge.** prism **already emits** `Reason::Resolution{kind}`
(types.rs:61-63, queries.rs:326); the harness just doesn't read it. Changes:
- prism: the §5 seam mints `InterfaceDispatch` (its `as_str` `"interface_dispatch"` rides into the
  existing `Reason::Resolution`). No new prism output plumbing.
- harness: `sut.py` (callers :78-89 / callees :92-105) extracts `Resolution.kind` from `why[]` into a
  new `CallEdge.resolution_kind` (model.py:36-41).

**(b) Fingerprinted in-scope interface-site manifest.** Reuse the existing fingerprint/re-anchor store
(adjudication.py:79-125; SHA256[:16] of the ±1-line window). Build a manifest keyed by `file:line` over
edges whose `resolution_kind == "interface_dispatch"`, recording `{seed_def, direction, fingerprint,
fallback_fired, inclusion_reason}`. **Inclusion** = an in-scope interface-dispatch edge (P6-lite-typed
receiver). Generated **after SUT extraction** (the metadata only exists once edges are resolved), persisted
to the run JSON. **Manifest-source limit (round-4 codex BLOCKER / GAPS):** a manifest keyed on *resolved*
`InterfaceDispatch` edges can record only sites that **did** resolve. The PR-2 receiver classes
(type-assertion/`var`/slice) **do not resolve today** — P6-lite never recovers their receiver, so they
produce **no edge** and are **invisible** to this stream. Therefore PR-1's manifest does **not** attempt to
enumerate PR-2 exclusions; the `§15` fatal gaps come from `interface_gaps` **telemetry**, and the PR-2
receiver-class exclusions require a **separate AST/drop-telemetry source** built in PR-2 (§ PR-2 work-list).

**(c) Per-site dispatch metadata — NOT a stratum (round-4 codex BLOCKER).** The rev-4 "add `I-dispatch`
to `STRATA` + pre-check `classify`" is **infeasible**: `classify` (strata.py:27-34) runs at **seed time**
(over the oracle inventory, cli.py:500) — *before* any SUT caller/callee edge exists — and receives only a
`FunctionDef`, so it cannot see a per-edge `resolution_kind`. Interface dispatch is an **edge/site**
property, not a seed property. So: **keep the five seed strata unchanged**; carry the attribute on the
edge (`CallEdge.resolution_kind`, set in (a) after SUT extraction) and on `Adjudication.dispatch_kind`
(adjudication.py:26-44); the precision gate (d) **filters the site-diff set by that per-site metadata**.

**(d) Precision gate (corrected).** At the manifest's **in-scope** interface sites:
**interface-dispatch-attributable FPs in `ExactOnly` = 0** — a *delta* gate on that set, NOT the
aggregate 0.81. If it trips, apply the §18 width-lever before any re-baseline. Fan-out-width telemetry
reviewed for outliers (`error`-class).

**(e) Acceptance scope for PR-1 (light — caddy-neutral by construction).** Because PR-1 resolves only
typed-param receivers (rare in caddy), the caddy corpus metric does **not** move: PR-1 acceptance is
`--matrix-only` + `--quick` + the §13.7 barrier fixture + the §13.11 harness unit test. **No 5-corpus
re-baseline and no caddy re-adjudication in PR-1** — those are PR-2's heavy ceremony (where the metric
actually moves). The apparatus built here is what makes PR-2's gate non-vacuous.

## 15. Gap taxonomy — fatal vs admitted (rev 5, round-4 codex MAJOR + claude note)

rev 4 lumped non-dispatchable cases and admitted over-approximations into one `GoDispatchGap` claiming
"recorded, **never matched**" — **contradictory**, because §6 admits bare-name cross-package matches and
§12's fallback admits non-local-construction edges, and both ARE matched (an Exact edge is minted). rev 5
splits the two:

**Fatal — `GoDispatchGap` (non-dispatchable; mints NO edge; truly never matched):**
```rust
pub enum GoDispatchGap {
    Generic,             // decl carries a type_parameter_list / interface type-set (§6, gated at decl)
    AnonymousInterface,  // non-empty anonymous interface (§6)
    UnknownCanonType,    // unenumerated canon_type node — fail closed (§6)
}
```

**Admitted — `GoDispatchOverApprox` (the Exact edge IS minted; a precision-risk counter, not a drop):**
```rust
pub enum GoDispatchOverApprox {
    CrossPackageBareName,          // io.Reader ≡ bufio.Reader under bare-name canon (§6 note)
    NonLocalConstructionFallback,  // empty-live fallback fired: full satisfier set, no RTA pruning (§8/§12)
}
```
Fatal gaps surface in `call-stats` `interface_gaps`; admitted over-approx in `interface_overapprox` +
`fallback_fired` (§10) — so §14's gate and PR-2 see **both** what the engine *declined* (fatal) and where
it *widened* (admitted). Tests (§13.3) assert each **fatal** path produces a gap and **no edge**; §13.7(ii)
exercises the **admitted** `NonLocalConstructionFallback` fan-out under ExactOnly.

## 16. DataFlow fan-out (Step-5b) — decision + telemetry (rev-4 MAJOR, claude r3-B3)

Step-5b (cpg/build.rs:384-526) re-resolves call sites into arg→param DataFlow edges feeding
taint/chop/delta. Interface dispatch with M live satisfiers × N args ⇒ **M×N** arg→param edges
(multiplicative). **Decision:** allow it — it is the **sound may-flow** counterpart of the M `Call(Exact)`
edges (same soundness class). The headline metric is **nav** (call edges), unaffected by DataFlow
volume. Guardrails: (1) fan-out width telemetry in `call-stats`; (2) §13.7 asserts the DataFlow fan-out
is **exactly** the satisfier set in **both** variants — live-intersection AND the empty-live fallback (no
non-satisfier leakage). A width-lever (demote very wide interfaces) is the §18 escape hatch only if §14 trips.

## 17. Step-9 CHA reconciliation (rev 5 — round-4 codex MAJOR; the rev-4 claim was WRONG)

rev 4 claimed CHA's override index is built from C++ classes so "a Go method is absent → CHA cannot mint a
Go edge." **That is false** (verified against build.rs:534-545): `virtual_method_nodes` is keyed by
**method name** and populated by scanning **every** `func_index` entry — all languages — for a name match
against a C++ `TypeDatabase` virtual-method name; the matched function's `NodeIndex` (**Go included**) is
inserted under the **C++ record's** class name. The seed loop (build.rs:547-552) likewise walks **all**
`Call(Exact)` edges, language-blind. So in a **mixed Go+C++ repo** a C++ Exact seed whose callee name
collides with a Go method *can* mint a spurious Exact `caller→Go-function` edge (mis-attributed with a C++
class) — and interface dispatch, by adding Go `Exact` edges (as both seeds and same-named override
candidates), could **widen** this pre-existing hazard.

**Fix (round-4 codex):** make CHA genuinely C++-only by filtering **both** the candidate index
(build.rs:536-543) **and** the seed scan (547-552) to functions the `TypeDatabase` actually owns (C/C++
source files). This strictly *removes* spurious cross-language edges; pure-C++ behavior is unchanged.
**Scope:** land it in PR-1 (small, and interface dispatch touches the same Exact-edge stream) even though
**no current corpus triggers it** — Go repos (caddy) have no `type_db`, so CHA never runs there; the
mixed-repo risk is future. Regression test: a synthetic mixed Go+C++ fixture with a name collision asserts
**no** cross-language virtual edge is minted.

## 18. Out of scope / deferred (engine-level)

- **Go generics / type-sets**, **anonymous interfaces**, **non-local-construction liveness**,
  **unknown `canon_type` nodes** — recorded `GoDispatchGap` (§15), never false matches.
- **Fan-out width-lever** — telemetry exists now (§16); a threshold-demote of very wide interfaces to
  NameOnly is added only if the §14 gate trips (owner: uncapped honest Exact by default).
- **Precise cross-package canonical keys / signatures** — bare-name over-approx now (§6 note); promote
  to package-qualified if §14/PR-2 shows material FPs.
- **Provider built twice** (CallGraph::build embedding + interface + registry) — same pure
  `from_parsed_files`; hoisting to a shared `Arc<GoTypeData>` is a deferred perf opt (build order makes
  it non-trivial). Accepted with this note.
- **`DispatchProvider` as the algorithm-facing API** — the registered provider's `resolve_dispatch`
  for slice algorithms with `CpgContext.live_types`; distinct from this resolver-internal consumption.

---

## PR-2 work-list (receiver expansion — committed fast-follow, its own plan)

PR-1 builds the engine + attribution for receivers P6-lite **already types**. PR-2 expands **which
receivers P6-lite types**, feeding the *same* `interface_impls` engine — **no engine rework**. This is
where caddy's *corpus* interface recall lives.

**Scope (owner-approved direction; confirm details at PR-2 brainstorming):**
- **Expand P6-lite receiver recovery** (the `receiver_type_in_fn` family, ast.rs:313-380) to:
  - **Type assertions** — `x.(Module).CaddyModule()` (the "57 caddy sites"). The asserted type is
    explicit in source → precision-safe (not the inferred return/field receivers S3 deferred).
  - **`var r Runner`** interface-typed locals (and short-var `r := factory()` where the static type is
    an interface).
  - **Interface-slice element receivers** — `for _, r := range runners { r.Go() }` where
    `runners []Runner`.
- Each new receiver-recovery case tags `ReceiverRecovery::{TypeAssertion, VarDecl, SliceElem}` so the
  §14 manifest can stratify the source of each interface edge.

**Heavy acceptance (PR-2 owns it — the apparatus is built in PR-1):**
- **Extend the §14 manifest** to the newly-recovered receiver classes. PR-1's resolved-edge manifest
  cannot see *unrecovered* receivers (§14b), so PR-2 must add the **separate AST/drop-telemetry source**
  that enumerates type-assertion/`var`/slice interface call-sites, then folds the now-resolved ones into
  the in-scope manifest (there is nothing to "move from exclusion" — PR-1 never recorded them).
- **Re-adjudicate** the now-resolved caddy interface sites (the "57" + `var`/slice) — dual-adjudicator,
  record κ; re-anchor via the fingerprint store.
- **§14 precision gate becomes meaningful** (interface-attributable FPs in `ExactOnly` = 0 over the
  expanded in-scope set); review fan-out-width outliers.
- **Full 5-corpus rerun** (`uv run tier-a --corpus all`) + **caddy anchor re-baseline** — deliberate,
  with the adjudication record. **This is the PR that should move the caddy metric.**

**Carried engine items that may need attention once real receivers flow:**
- **Factory-return / non-local liveness** (§8 gap) — if §14 shows the fallback over-approximating on
  real caddy data, implement non-local construction liveness (return-type scan or constructor registry)
  here rather than in PR-1.
- **Precise cross-package keys** (§6 note) — if §14 shows cross-pkg bare-name FPs.
- **Fan-out width-lever** (§18) — only if the expanded gate trips.

**Deferred beyond PR-2 (unchanged):** Python inheritance, `from_import_alias`, Rust S3.1 (the prism
C-method recall lever 0.121), Go generics/type-sets, anonymous interfaces.
