# S3 — Call-Resolution Precision Floor — Design

**Date:** 2026-06-12 · **Status:** rev 1 — owner-approved in brainstorm (scope, blast
radius, acceptance bar, approach, and all four design sections approved interactively).
**Context docs:** `docs/eval/tier-a/baseline.md` (the measured S3 work-list — the
anchor), `docs/cpg-substrate-analysis-2026-06-10.md` §3 S3 row (prescribed fix shape),
`docs/prism-query-layer/tier-a-handoff-2026-06-12.md` (sequencing: S3 → S2 → Plan B).

## 0. Why now

The Tier-A baseline measured what the substrate analysis predicted: prism's
call-resolution precision floor is the dominant accuracy defect visible to agents.

- **Precision (922 prism_fp records):** collision-prone method names are claimed across
  receiver types at devastating scale — tokio collision-method callers corrected
  **P = 0.00 with 390 FPs** (`poll`/`as_fd`/`write`); caddy 441 FPs (`Error`). Stdlib
  methods bind in-corpus (`Vec::truncate` → `AccessPath::truncate`).
- **Recall (215 prism_fn records):** qualified `Type::fn` / `mod::Type::fn` calls are
  missed (the capability matrix's `type_method_qualified` known_fail) — the dominant
  U-callee class (recall 0.70).

Both classes share one missing fact: **which type owns each method definition**. S3
adds that fact and rebuilds the shared resolver around it.

## 1. Scope and decisions (owner-approved 2026-06-12)

| Decision | Choice |
|---|---|
| Scope | Work-list **items 1+2 only**: collision-method caller FPs + qualified `Type::fn` binding. Constructors (3), decl→impl seed mapping (4, harness-side), nested-def attribution (5) are follow-ups. |
| Blast radius | **Shared resolver** (`CallGraph::resolve_callees_qualified`) — slicing, diff review, nav, and MCP all inherit the fix; Plan B inherits truthful boundaries. Slicing goldens drift in the improving direction and are re-blessed with review. |
| Acceptance bar | **Directional + no regressions**: collision-method FP class near-eliminated, `type_method_qualified` flips to pass, zero metric regressions elsewhere, honest report of the residue. No hard numeric gate (avoids over-tightening into false negatives). |
| Approach | **B — method-owner index + tiered confidence** (over heuristics-only A and full type-confirmed dispatch C, which stays Phase-IP). |
| Confidence storage in CPG | **Deferred to S2** (§7). The resolver *returns* confidence (the seam); the CPG does not store it this phase — slicing output is line sets, so include/exclude (decided in the resolver) is the only distinction that reaches it. |

## 2. The resolution model

### 2.1 The new fact: method owners

`FunctionInfo` (src/ast.rs) gains `owner: Option<String>` — the type that owns a
method definition. Populated in `build_function_table` via an ancestor walk from each
function node; per-language owner node kinds live in `src/languages/mod.rs` (new
`Language` method, e.g. `method_owner(node) -> Option<String>`).

| Language | Owner source |
|---|---|
| Rust | Enclosing `impl_item` type (generics stripped: `impl<T> Foo<T>` → `Foo`). `impl Trait for Type`: indexed under **both** `Type` and `Trait` (dual-key) — `Trait::m(..)` UFCS calls resolve to all impls (sound CHA-style over-approximation), `Type::m` stays precise. Trait default methods (`trait_item`) → owner = trait name. |
| Go | `method_declaration` receiver type, `*` stripped (`func (t *T) m` → `T`). The receiver **variable name** (`t`) is also captured (`FunctionInfo.receiver_var: Option<String>`, `None` for all other languages) — R2 needs it to recognize `t.m()` inside the method body as a receiver-var call; Rust/Python/JS use the literal keywords `self`/`this`. |
| Python / JS / TS / Java | Enclosing class, **only if the function is a direct member** — a nested `def` inside a method gets `None`, not the class (keeps the descoped nested-def-attribution problem out of the index). |
| C++ | In-class definitions + out-of-line `Foo::bar` qualified declarators → `Foo`. Namespace-qualified free functions keep `None` (R1 falls through to stem matching for `ns::f()`). |
| Lua | `function M.f()` / `M:f()` → `M`. |
| C / Bash / Terraform | Always `None`. |

JS object-literal methods (`const obj = {m(){}}`) are `None` in v1 — noted, not
attempted. Parse-degraded files degrade to `None` (method looks like a free function);
the existing parse-warning machinery covers this, no new failure mode.

CallGraph Phase 1 builds the owner index alongside the existing name map:
`methods: BTreeMap<(String, String), Vec<FunctionId>>` keyed by `(owner, name)`.
`FunctionId` itself is unchanged — identity work is S2's, not S3's.

### 2.2 The rule ladder

`resolve_callees_qualified` is restructured into an ordered ladder. Return type becomes
`Vec<(&FunctionId, ResolutionConfidence)>` with a two-variant enum:
`Exact` | `NameOnly`.

| # | Call shape | Resolution | Confidence |
|---|---|---|---|
| R1 | Qualified `T::m`, `mod::T::m` (Rust/C++) | Owner-index lookup `(T, m)`; when the path carries module segments that match a candidate file's stem/module path, restrict to that file. For module-qualified (non-type) paths the existing stem-match fallback applies — it **stays nav-side** (`call_resolve.rs`, where it lives today; the CPG path never had it) and now runs only when the shared ladder returns no candidates — still Exact when the stem resolves. | **Exact** |
| R2 | `self.m()` / `Self::f()` / `this.m()` / Go receiver-var call | Enclosing function's owner type → `(owner, m)`. Works across split impl blocks/files (index is repo-wide). | **Exact** |
| R3 | Module-qualified `pkg.f()` via imports map | Unchanged (existing behavior). | **Exact** |
| R4 | Unqualified `f()`, caller's file defines `f` as a **free function** | That definition alone (local-definition preference). A local *method* named `f` does not satisfy R4 — unqualified calls never bind to methods (Rust needs `Self::`, Python needs `self.`). | **Exact** |
| R5 | Unqualified `f()`, cross-file | Free-function candidates only — **method candidates are excluded** (a method cannot be called without a receiver). Single candidate → Exact; multiple candidates → all kept, demoted (edge set unchanged from today minus methods; only scores move). | **Exact** / **NameOnly** |
| R6 | Receiver call `x.m()`, receiver type unknown (qualifier present, not an imported module, not self/this) | Method candidates only, never free functions. First check the **caller's own file**: exactly one owner defining `m` there → resolve there, demoted. Else: **one owner repo-wide** → keep, demoted; **multiple owners** → *unresolved* (prefer unresolved over wrong). | **NameOnly** (demoted) / dropped |

How this addresses the measured classes:
- tokio `poll`/`as_fd`/`write` are multi-owner → **dropped** (R6). The 390-FP class
  disappears from caller claims rather than appearing demoted — the edge is gone.
- `Vec::truncate` → `AccessPath::truncate` (stdlib receiver, single in-repo owner)
  survives as **demoted + labeled**, not silently asserted; the corpus rerun measures
  the size of this residue.
- `Type::fn` where the type name differs from the file stem resolves exactly (R1) —
  flips `type_method_qualified`. `Self::`/`self.` fan-out becomes exact (R2).

### 2.3 Known recall cost, stated honestly

Go interface dispatch and Rust trait-object calls with unknown receivers are
multi-owner → dropped. The baseline showed some of those edges are *true* (the caddy
interface-dispatch adjudication record — prism had the edge at the concrete impl).
This is the approved trade; the corpus rerun measures the net, and type-confirmed
dispatch (Phase-IP, E12 `DispatchProvider`) is the recovery path. Dropped resolutions
surface through the existing unresolved-callee representation — absent edges are
visible, not silent.

## 3. Components and data flow

1. **`src/ast.rs`** — `FunctionInfo.owner`, populated in `build_function_table`
   (ancestor walk). **`src/languages/mod.rs`** — per-language `method_owner`.
2. **`src/call_graph.rs`** — Phase 1 `methods` index; `resolve_callees_qualified`
   becomes the R1–R6 ladder returning `(FunctionId, ResolutionConfidence)`. Receiver
   calls are already distinguishable from the existing `qualifier` + imports map; the
   only extraction additions are `owner` and Go's `receiver_var` (§2.1). R2 needs the
   enclosing function's owner, which Phase 2 has.
3. **`src/cpg/build.rs` Step 5/5b** — same resolver; inherits narrowed candidate sets.
   Confidence is **not stored** (petgraph edges unchanged in shape). Step 5b arg→param
   edges stop polluting across false edges — the Plan B payoff.
4. **`src/navigation/`** — `call_resolve.rs`'s `::`-stem fallback is mostly subsumed
   by R1 and retained nav-side as the last-resort tier, running only when the shared
   ladder returns no candidates (§2.2 R1). `queries.rs` / `module_graph.rs` map
   confidence → `score` (**Exact = 1.0, NameOnly = 0.5**) × existing hop decay;
   `module-deps`/`repo-map` aggregate per file-pair, max-score-wins. New **additive**
   `Reason::Resolution { kind }` on `why` (serde-additive; Evidence stays
   omit-when-absent compatible). Scores stop being uniformly 1.0.
5. **Caches** — resolver changes alter edge sets ⇒ one `CACHE_VERSION` bump (nav
   store). S1 made cold rebuilds cheap; the old bump taboo does not apply.

**Data flow:** parse → `FunctionInfo{owner}` → Phase 1 owner index → Phase 2 call
sites (unchanged shape) → R1–R6 with confidence → (a) CPG edges (narrowed, no
confidence) for the 26 algorithms + diff review; (b) nav Evidence with tiered scores
and resolution reasons for CLI + MCP.

**Compat posture:** diff-review output is *expected* to drift where false edges
disappear — goldens re-blessed with review (§5). Evidence JSON changes are additive
plus score-value changes on the demoted class. MCP tool schemas unchanged.

## 4. Edge cases

- `Self::f()` inside a trait default method: owner = trait; `(trait, f)` hits trait
  defaults and (via dual-key) impls.
- Rust split impls across files: repo-wide `(owner, m)` index resolves R2 cross-file.
- Python `super().m()` and other dynamic receivers: fall to R6 — noted gap, Phase-IP.
- Multiple impl blocks for one type: multiple `Vec` entries under one key — fine.
- Anonymous functions (`name == None`): owner irrelevant, never indexed.

## 5. Testing and acceptance

1. **TDD throughout** (subagent execution per workflow preferences): per-language
   owner-extraction unit tests (`tests/ast/`), resolver rule tests R1–R6 with
   confidence assertions, nav score/`Reason` tests.
2. **Capability matrix:** `type_method_qualified` flips known_fail→pass (status
   updated in the same PR). New minimal **collision fixtures** for Rust + Go
   certifying the R6 drop and single-owner demotion (the sliver of followup 8 that
   S3's own claims need; Python stays matrix-protected).
3. **Goldens:** slicing/diff-review drift re-blessed with review — every drifted line
   must trace to a removed false edge or a newly-resolved qualified call; unexplained
   drift is a regression.
4. **Harness protocol** (CLAUDE.md): `cargo build --release` + `--matrix-only`
   pre-commit; `--quick` before review; **acceptance = full 5-corpus rerun**
   (human-triggered), before/after vs `docs/eval/tier-a/baseline.md`:
   collision-class FPs near-eliminated, qualified-call recall class closed, zero
   metric regressions elsewhere. Flip-candidates pasted into the PR description;
   `baseline.md` updated deliberately after acceptance. Stale line-keyed
   adjudications handled per the dual-adjudicator protocol if new diffs appear.
5. **Perf guard:** owner index is Phase-1 work plus one ancestor walk per function —
   no measurable build-time regression expected; the P1/P2 timing protocol exists if
   numbers look off.

## 6. Risks and mitigations

| Risk | Mitigation |
|---|---|
| Over-tightening creates false negatives (the R2 register's named risk) | Tiered fix: drop only multi-owner ambiguity; single-owner kept demoted; recall measured by the same acceptance rerun that measures precision — "no regressions" is two-sided. |
| Golden churn hides a real regression | Re-bless discipline (§5.3): every drifted line explained or the drift fails review. |
| Owner extraction wrong in a language → silent misclassification into R5/R6 | Per-language TDD fixtures first; parse-degraded files degrade to `None` (existing warning machinery). |
| Score change surprises Evidence consumers | Only the demoted class moves off 1.0; `Reason::Resolution` names why; MCP schemas unchanged. |

## 7. Deferred / follow-ups (created or reaffirmed by this spec)

1. **Confidence on CPG call edges → S2** (owner-approved deferral): store
   `ResolutionConfidence` on `CpgEdge::Call` during S2's batched type churn + cache
   bump; first consumers are Plan B boundary honesty and `gradient_slice`.
2. Work-list items 3 (constructor edges), 4 (decl→impl seed mapping, harness-side),
   5 (nested-def attribution) — next precision/recall passes after S3 measures.
3. Type-confirmed dispatch via E12 `DispatchProvider` (Phase-IP) — recovers the
   interface/trait-object recall cost in §2.3.
4. JS object-literal method owners; Python `super()` resolution.
5. Matrix v2 collision-rich fixtures beyond the Rust/Go sliver (tier-a-followups 8).
