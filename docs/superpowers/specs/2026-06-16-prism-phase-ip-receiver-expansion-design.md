# Phase-IP Go interface dispatch — receiver-expansion (PR-2) — DESIGN

> **rev 2 (2026-06-16) — dual spec-review folded (codex gpt-5.5 xhigh + claude opus).** Both:
> strategically sound (engine-vs-recovery split right; no-build/no-SCIP posture aligned with
> `docs/features/cpg/substrate-analysis-2026-06-10.md` / `docs/archive/analysis/prism/prism-meta-analysis-2026-06-10.md`; S3 receiver-recovery
> component, not a parallel layer), with buildability/spec gaps now fixed. Key folds: **recover-and-route**
> (recover the receiver's static type by syntactic fact; the existing `owner_lookup → interface_impls →
> drop` ladder routes it — no `GoTypeProvider` at extraction, no interface-set predicate; resolves the
> rev-1 var-local blockers + the §3↔§12 contradiction); the seam is **`recover_receiver`
> (`call_graph.rs:1384`, two call sites)** + the `ast.rs` scanners, and it must be **fed the receiver
> node** (today it gets only qualifier text, and type-assertions are rejected by its simple-ident gate) —
> a real extraction-API change; **Slice A (seam refactor, byte-identical `legacy` parity) is a mandatory
> pre-PR**; grammar **pinned** (`type_assertion_expression`); manifest keyed by **byte-span**; gate FP rule
> + config surface specified. rev-1 in git history (`2076e05`).
>
> **PR-1 is MERGED** (`5cd1ac9` on `main`, #96). This branch (`phase-ip-receiver-expansion`) is stacked
> directly on it.

## §0 — Context & goal

Phase-IP is type-confirmed Go receiver dispatch — the **recall** counterpart to EFT's precision. The
**embedding** half shipped (#95); the **interface FOUNDATION (PR-1)** merged (#96): it resolves `r.Go()`
where `r` is a P6-lite-**already-typed** receiver (typed param / constructor local) to in-repo implementers
of interface `Runner`, minted `Exact`/`InterfaceDispatch`, via signature-confirmed structural satisfaction
+ RTA liveness + a kept-Exact empty-live fallback, gated to Go callers.

**PR-2 expands which receivers carry a recovered static type**, so more interface-method call-sites reach
PR-1's dispatch engine. **The engine does not change.** Per the recover-and-route model (§2), recovery is
purely syntactic and the *existing* resolution ladder routes it — so PR-2 is "recover more receiver
types," nothing in `compute_interface_dispatch` / satisfaction / the seam moves.

**Why this moves caddy.** caddy's interface recall lives in **57 diff sites** (~19 distinct ×3 `CaddyModule`
implementers), **all type-assertion receivers** (`x.(Module).CaddyModule()`), today adjudicated `ambiguous`
(`docs/eval/tier-a/re-anchor-adjudication-2026-06-14.md:84`). P6-lite doesn't recover an asserted receiver,
so PR-1 is caddy-corpus-neutral. PR-2 recovers them → resolvable → caddy interface recall can move (after
re-adjudication + a deliberate re-baseline). *Metric movement is a spec expectation, not yet remeasured.*

**Dependency:** PR-1 (`main`). No other hard dependency (§11).

## §1 — Scope

**In PR-2 scope (implemented):**
1. **Type-assertion receivers** — `x.(Module).CaddyModule()` (all 57 caddy sites).
2. **`var`-declared local receivers** — `var r Runner` / `var r Runner = f()` (recover-and-route: recover
   the *declared* type whether interface or concrete; §2 routes it).

**Sketched (designed-for behind the seam, not implemented):**
3. **Interface-slice element receivers** — `for _, r := range runners` where `runners []Runner` (§5).

**Deferred-conditional** (activate only if the §8 gate *report* shows that FP/recall class —
measure-then-decide): precise cross-package keys (D2), non-local/factory-return + return-type-inferred
recovery (D4), the fan-out width-lever. **Stays deferred:** pointer-embed admission (D5), CHA abs/rel path
(D6), Python inheritance, Rust S3.1, Go generics/type-sets, anonymous interfaces.

## §2 — The receiver-classifier seam (core abstraction)

**Where recovery actually lives today (corrected from rev 1).** The orchestrator is
`CallGraph::recover_receiver` (`src/call_graph.rs:1384`), called from **both** extraction paths
(`build` ~`:373`, `build_direct_subset` ~`:1023`); it applies the qualifier/keyword/import/`recv_var` gates
(`:1398-1411`) then delegates the type-text scan to `ParsedFile::receiver_type_in_fn` /
`walk_receiver_bindings` (`src/ast.rs:313/3816`, which use private helpers `find_parameters_node`,
`constructor_type`, `simple_binding_text`, …). `CallSite.receiver_type`/`receiver_recovery` are assigned at
**two** sites (`call_graph.rs:~388`, `~1038`). The seam must subsume `recover_receiver` (both call sites),
and `legacy` must preserve its **gate logic** verbatim, not just the scanners.

**Extraction-API change (load-bearing, both reviewers).** `recover_receiver` today receives only the
**qualifier string** + call span; type-assertions are rejected by its simple-identifier gate. To recover
`x.(Module).M()` the classifier needs the **receiver/selector node** (or the call node) — so the call
extractor (`function_calls_with_qualifier_and_spans_on_lines`) must additionally surface the receiver
expression node into `ReceiverCtx`. This is the one real interface change PR-2 makes upstream of the engine.

**The seam.** A `ReceiverClassifier` with two implementations:
- `legacy` — the current behavior (gates + `TypedParam`/`ConstructorLocal` scans), extracted **verbatim**.
- `expanded` — `legacy` ∪ the new forms (type-assertion, var-local; reserved slice).

```rust
// src/resolution.rs  (S3 vocabulary; called by call_graph.rs extraction)
pub struct RecoveredReceiver { pub static_type: String, pub recovery: ReceiverRecovery }

pub struct ReceiverCtx<'a> {
    pub receiver_expr: tree_sitter::Node<'a>, // NEW: the receiver/selector node (not just qualifier)
    pub qualifier: Option<&'a str>,           // existing
    pub fn_node: tree_sitter::Node<'a>,        // enclosing fn (existing)
    pub call_line: usize,                      // existing
    pub parsed: &'a ParsedFile,                // for node_text / helper reuse
    // NOTE: recover-and-route needs NO GoTypeProvider here (see §4).
}
pub trait ReceiverClassifier { fn classify(&self, ctx: ReceiverCtx<'_>) -> Option<RecoveredReceiver>; }
```

**Recover-and-route (the unifying model).** The classifier recovers a **syntactic** static type and tags
the *fact* (`TypedParam`/`ConstructorLocal`/`TypeAssertion`/`VarDecl`/reserved `SliceElem`). It performs
**no** interface-vs-concrete test. The existing ladder routes: `owner_lookup(recovered, m)` resolves a
concrete type (a recall win, same precision class as typed-params) and only on a **miss** does
`iface_key → interface_impls` dispatch (where a concrete type has no entry → drop). A concrete type
therefore cannot receive a wrong interface edge. This eliminates any need for the `GoTypeProvider`
interface set at extraction (the rev-1 var-local blockers B1/B2) and removes the §3↔§12 concrete-asserted
contradiction.

**Wire/cache (corrected).** `ReceiverRecovery` gains additive variants `TypeAssertion`, `VarDecl`, and a
reserved `SliceElem`. No `CallSite` struct change. Enum additions are cache read-compatible if existing
discriminants stay stable; `CallGraph` is serialized into the CPG cache (`cpg_cache.rs`), and cache
validity already includes `GIT_SHA` (`cpg_cache.rs:296-307`), so a *built* binary self-invalidates per
commit — **dirty-vs-dirty dev iteration shares `-dirty` and needs `--no-cache`** (existing discipline,
`cpg_cache.rs:296-300`). Bumping `CACHE_VERSION` 9→10 when the variants land is a one-line option to also
close the dirty hole (matches PR-1's v8→v9 precedent); recommended but not required. The **output** stays
`ResolutionKind::InterfaceDispatch` (for interface hits) or the existing owner-index kind (concrete) —
`ReceiverRecovery` is the input fact, not a new output kind.

## §3 — Form 1: type-assertion receivers (`x.(Module).CaddyModule()`)

Grammar (**pinned**, tree-sitter-go 0.23.4): the call is `call_expression` whose `function` is a
`selector_expression` whose `operand` is a `type_assertion_expression` with fields `operand` (the asserted
expression) and `type` (the asserted type). Precedent: `taint.rs:5567` already walks a type-assertion via
`child_by_field_name`. The classifier, when the receiver node is a `type_assertion_expression`, recovers its
`type` child (owner-normalized: strip `*`, take the bare segment of `pkg.Module`, unwrap a parenthesized
type), tagged `TypeAssertion`. **Excluded:** the comma-ok form `v, ok := x.(T)` — that is an assignment, not
a method-call receiver, so it never enters the classifier (it is not in receiver position of a call). The
recovered type routes per §2 (interface → dispatch; concrete `x.(*Foo).Bar()` → `owner_lookup`). Pin all of
`Module` / `pkg.Module` / `*T` / `(T)` with tests against the pinned grammar (the PR-1 6-canon-facts
precedent).

## §4 — Form 2: `var`-declared local receivers (`var r Runner`)

Recover the **declared static type** of a local whose receiver is `r`, tagged `VarDecl`, and route per §2 —
**no interface predicate, no `GoTypeProvider` at extraction.** An interface-typed local dispatches via
`interface_impls`; a concrete-typed local resolves via `owner_lookup` (recall win); neither needs a type
test. Binding coverage (reuse the existing "bindings at/before call; >1 binding bails" rule at
`ast.rs:326-379`/`3828-3887`, extended to `var`): `var r Runner`, `var r Runner = f()`, `var ( r Runner; … )`
blocks, and package-level `var r Runner`. Multiple/reassigned/shadowed bindings → **bail** (recover nothing,
unchanged behavior), as today. The bare `r := f()` (no annotation) is *return-type-inference*-dependent and
stays in D4/deferred (the boundary toward the sketched/conditional work). (Naming: `VarDecl` supersedes
rev-1's `InterfaceLocal` and matches the PR-1 work-list; recover-and-route makes "is it an interface" a
routing outcome, not a recovery predicate, so the syntactic name is correct.)

## §5 — Form 3 (SKETCH ONLY): interface-slice element receivers

`for _, r := range runners { r.Go() }`: `r`'s static type is the slice's element type. **There is no
existing element-type recovery helper** (`canon_type`'s `slice_type` handling is signature
canonicalization, a different concern). Implementing `SliceElem` requires a **new intra-procedural
range-source declared-type resolver** (find `runners`'s declared `[]Runner`, take the element type) — the
same binding-resolution shape as the constructor-local case, scoped **declaration-site only** (return-type
inference stays in D4). The seam **reserves** `ReceiverRecovery::SliceElem`; the classifier case is stubbed
(`None`). For §8 manifest stratification, unrecovered slice-receiver sites are enumerated as a
**manifest-only candidate class** (the AST scan can recognize the `range` shape even though the classifier
recovers nothing), so the variant not appearing on call sites does not hide them.

## §6 — Dispatch flow (unchanged, inherited from PR-1)

For a recovered interface type `I` + method `m`: `owner_lookup(I,m)` misses (interfaces have no owner-index
bodies) → `iface_key(I)` → `interface_impls[(I,m)]` → mint N `Exact`/`InterfaceDispatch` callees;
signature-confirmed satisfaction (`set_ptr ⊇ set_value`, promoted, generics/embedded gaps), RTA pruning to
the live admission set, kept-Exact empty-live fallback, Go-language gate — all unchanged. A recovered
**concrete** type resolves at `owner_lookup` and never reaches the interface consult. **No engine change.**

## §7 — Confidence & honesty

Inherited: `Exact` only when signature-confirmed (interface) or owner-index-resolved (concrete); otherwise
the kept-Exact fallback (no silent drops); "label confidence, don't delete edges." PR-2 adds no new
confidence rule. If the §8 report shows a recovery form is a precision liability, the measure-then-decide
response (width-lever / cross-package keys / a confidence downgrade) is chosen then, not pre-committed.

## §8 — Acceptance: in-scope manifest + precision gate (as a report)

PR-1 already extracts + persists + replays per-edge `resolution_kind`/`dispatch_kind`
(`sut.py:78-108`, `test_replay_keeps_resolution_kind.py`). PR-2 owns the deferred pieces:

**(a) In-scope manifest (the §14b separate AST source).**
- **Denominator predicate (defined):** an interface-method call-site is **in-scope** iff its receiver is one
  of the recognized forms — typed-param, constructor-local, type-assertion, `var`-local, or
  range/slice-element — **AND** the called method name appears on **some known interface** in
  `GoTypeProvider` (so the manifest counts genuine interface-dispatch candidates, not every method call).
  The AST scan recognizes the receiver shape syntactically; the "method ∈ some interface" check uses the
  provider at *manifest-build* time (post-extraction, where the provider exists — not the extraction-time
  constraint §4 avoids).
- **Identity (corrected): byte-span keys.** Key each site by `file:start_byte:end_byte` (the `CallSite`
  already carries + orders by byte spans, `call_graph.rs:27-31/1347-1357`), with `file:line` as display
  only. Fingerprint with the existing `adjudication.fingerprint` window-hash for drift re-anchoring.
- **Stratify** by receiver class (incl. the §5 manifest-only `slice-candidate`).

**(b) Precision gate — a REPORT, not a hard fail (owner: measure-then-decide).** Over the manifest's
in-scope sites, emit per-receiver-class JSON: `{corpus, direction, receiver_class, dispatch_sites,
concrete_sites, raw_fp, corrected_fp, pending, ambiguous, fanout_width}`. **FP computation rule (defined,
as-built — supersedes the original subtractive wording, per the whole-branch re-review):** the FP metric is
over interface-**dispatch** sites only (`fanout > 0`); concrete owner-resolved receivers (`fanout == 0`) are
reported as `concrete_sites` and excluded from the FP counts. `raw_fp` = prism-only dispatch candidates (the
pre-adjudication upper bound; the oracle-derived prism-only set narrows the FP numerator in Slice E, all
in-scope in PR-2). `corrected_fp` = **positive selection** of sites adjudicated `prism_fp` only — `oracle_miss`
is a TP (prism was right), `alias_site`/`ambiguous`/`oracle_artifact` are excluded, and an unadjudicated site is
`pending`. The denominator fields (`dispatch_sites`/`concrete_sites`/`fanout_width`) are over all in-scope class
sites; the FP numerator over the prism-only subset. `pending` = unadjudicated. The gate is reported, never gating, in PR-2. **Oracle dependency:** the
denominator + stratification + `fanout_width` run in **Slice D alone** (structural, no oracle); the
`corrected_fp` line is meaningful only **after Slice E** re-adjudicates the now-resolved sites (against
existing `adjudications.jsonl` it is labeled *provisional*). After the first real run we read it and decide
any tightening together.

## §9 — caddy re-adjudication + re-baseline (human-gated)

Once PR-2 resolves the 57 sites: (1) **re-adjudicate** via the dual-adjudicator protocol (codex + claude),
record Cohen's κ, re-anchor stale verdicts by fingerprint; (2) **re-baseline** caddy — full 5-corpus rerun
(`uv run tier-a --corpus all`) + deliberate anchor update in `docs/eval/tier-a/`, with the adjudication
record. **Human-gated** (multi-corpus runs are human-triggered). This is the PR that should move the caddy
metric; the move is recorded, not silent.

## §10 — Incremental check-ins (slicing)

- **Slice A — MANDATORY PRE-PR (both reviewers):** the `ReceiverClassifier` seam + `legacy` impl + the
  extraction-API change to pass the receiver node. **Pure refactor, byte-identical resolution**, guarded by
  PR-1's P6-lite fixtures as the parity gate. Lands alone (clean bisect) before any new form.
- **Slice B:** `TypeAssertion` form + tests + a `go/interface_dispatch_assert` tier-A fixture.
- **Slice C:** `VarDecl` form + tests + a `go/interface_dispatch_var` fixture.
- **Slice D:** the manifest source + the gate report.
- **Slice E (human-gated):** caddy re-adjudication + re-baseline.
- Slice F (`SliceElem`) sketched only.

Each form is independently revertable to `legacy` via the config (§13).

## §11 — Substrate alignment & prerequisites

**Alignment (reviewer-confirmed, no drift).** PR-2 is the **S3 receiver-recovery component** of the
`S1→S2→S3→S4` floor (`docs/features/cpg/substrate-analysis-2026-06-10.md`): it tightens the **shared** resolver
(benefiting diff-review, nav — verified nav is always whole-repo — and Tier-2 Step-5b together), extends
the existing `ReceiverRecovery` vocabulary (not a parallel one), and feeds Phase-IP dispatch. It honors the
no-build/all-text load-bearing property (`docs/archive/analysis/prism/prism-meta-analysis-2026-06-10.md`): recovery is **syntactic**,
never compiler-dependent; `Source::ExternalIndex`/SCIP stays a *future* arbitration seam, not a PR-2
dependency. "Label confidence, don't delete edges" is preserved (kept-Exact fallback; concrete misses drop
exactly as today).

**Prerequisites:**
- **PR-1 (`main`) — satisfied** (merged #96, `5cd1ac9`). S1/S2/S3/E12/embedding all in `main`.
- **Slice A first** (§10) — the seam refactor + node-passing API is the implementation prerequisite for the
  forms; it is *in* PR-2 (its first slice), not a separate dependency.
- **No node-identity gap:** the manifest's byte-span keys use spans `CallSite` already carries (S2 landed),
  so no new AST-bytes work is needed. The Tier-A `Location` model is line-keyed today; the manifest can be
  byte-stricter than the harness without changing the harness `Location`.

## §12 — Testing

- **Classifier unit tests** (per form): `TypeAssertion` recovers the asserted type (`Module`/`pkg.Module`/
  `*T`/`(T)`; comma-ok excluded); `VarDecl` recovers the declared type for interface **and** concrete
  locals, bails on multi/shadow bindings; `SliceElem` `#[ignore]`d placeholder pinning the reserved variant.
- **`legacy` parity test:** `legacy` reproduces PR-1's recovery byte-for-byte on the existing P6-lite
  fixtures (the strangler-swap guard) — the Slice A gate.
- **Resolution tests:** `x.(Module).M()` + `var r Runner; r.M()` → in-repo implementers
  `Exact`/`InterfaceDispatch`; `x.(*Concrete).M()` + `var r Concrete; r.M()` → `owner_lookup` resolution;
  Go-language gate still blocks cross-language.
- **tier-A fixtures:** `go/interface_dispatch_assert`, `go/interface_dispatch_var`.
- **Harness:** manifest denominator/stratification + byte-span-key test; gate-report computation test
  (raw/corrected/pending/ambiguous fields).
- **Config tests:** `legacy`, `type_assertion_only`, `var_local_only`, full `expanded` selection.

## §13 — Resolved decisions

1. **Recovery model:** recover-and-route (§2) — owner-confirmed.
2. **Form-2 interface predicate:** none (routing handles it) — kills B1/B2.
3. **Config surface:** a `ReceiverRecoveryMode` enum (`Legacy` | `Expanded`) **plus per-form booleans**
   (`type_assertion`, `var_local`) on the build config; **default `Expanded`** (all implemented forms on).
   `Legacy` is the granular fall-back. (§2/§13 rev-1 contradiction resolved → default is `Expanded`.)
4. **Slice A:** mandatory pre-PR (§10).
5. **Slice-element source:** declaration-site intra-procedural only (return-type inference → D4).
6. **Cache:** GIT_SHA covers built; dirty needs `--no-cache`; `CACHE_VERSION` 9→10 optional-recommended.

## §14 — Open decisions (measure-then-decide)

1. Gate response if it trips (§8b): cross-package keys vs. width-lever vs. documented threshold — after the
   first report.
2. Whether to bump `CACHE_VERSION` (§13.6) — decide when the variants land.

---

### Self-review (author, rev 2)

- **Placeholders:** none. Grammar is pinned (§3); the `ReceiverClassifier`/`ReceiverCtx` shapes are
  illustrative-but-anchored (named files/lines verified by review).
- **Consistency:** recover-and-route stated identically in §2/§3/§4/§6; `ReceiverRecovery` = input fact
  (additive), `ResolutionKind` unchanged; §1 scope ↔ §10 slices ↔ §12 tests ↔ §13 decisions aligned;
  §3↔§12 (concrete asserted) and §2↔§13 (default config) contradictions resolved.
- **Scope:** one plan — Slice A (seam+API) + 2 forms + manifest/report + (human-gated) re-baseline; slice
  sketched; tightening deferred-conditional.
- **Ambiguity:** "report not gating" (§8b) and "human-gated re-baseline" (§9) stated explicitly; the FP rule
  + oracle dependency defined; manifest identity is byte-span (not line).
