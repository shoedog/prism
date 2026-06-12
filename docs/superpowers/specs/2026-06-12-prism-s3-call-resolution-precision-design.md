# S3 — Call-Resolution Precision Floor — Design

**Date:** 2026-06-12 · **Status:** rev 2 — dual cleanroom review folded (codex gpt-5.5
xhigh rigor + claude fable xhigh soundness, R6 redacted for both; record:
`docs/prism-query-layer/s3-spec-review-2026-06-12.md`). Both reviewers independently
recommended the R6 policy now in §2.3; rev 1 was owner-approved in brainstorm.
**Context docs:** `docs/eval/tier-a/baseline.md` (the measured S3 work-list — the
anchor), `docs/cpg-substrate-analysis-2026-06-10.md` §3 S3 row (prescribed fix shape),
`docs/prism-query-layer/tier-a-handoff-2026-06-12.md` (sequencing: S3 → S2 → Plan B).

## 0. Why now

The Tier-A baseline measured what the substrate analysis predicted: prism's
call-resolution precision floor is the dominant accuracy defect visible to agents.

- **Precision (922 prism_fp records¹):** collision-prone method names are claimed
  across receiver types at devastating scale — tokio collision-method callers
  corrected **P = 0.00 with 390 FPs** (`poll`/`as_fd`/`write`); caddy 441 FPs
  (`Error`: ≈167 receiver-var sites + ≈148 imported-package-qualified sites that are
  an *import-narrowing fall-through* class, not a receiver class — see R3). Stdlib
  methods bind in-corpus (`Vec::truncate` → `AccessPath::truncate`, cross-file).
- **Recall (215 prism_fn records):** qualified `Type::fn` / `mod::Type::fn` calls are
  missed (the capability matrix's `type_method_qualified` known_fail) — the dominant
  U-callee class (recall 0.70).

Both classes share one missing fact: **which type owns each method definition**. S3
adds that fact and rebuilds the shared resolver around it.

¹ Post-amendment `eval/adjudications.jsonl` counts are 924 prism_fp / 37 oracle_miss;
the prose keeps baseline.md's header numbers and notes the delta here.

Measured scale (operator census, regex-estimated, recorded for design rationale):
prism — 9,106 non-self `x.m()` sites, 17% hit in-repo method names (16% single-owner,
**1% multi-owner**); tokio — 67% hit in-repo names (17% single, **50% multi**); method-
name collision rates prism 11%, caddy 18% (top: `UnmarshalCaddyfile` ×93, `Provision`
×91 — interface-shaped), tokio 30%.

## 1. Scope and decisions

| Decision | Choice |
|---|---|
| Scope | Work-list **items 1+2 only**: collision-method caller FPs + qualified `Type::fn` binding. Constructors (3), decl→impl seed mapping (4, harness-side), nested-def attribution (5) are follow-ups. |
| Blast radius | **Shared resolver** (`CallGraph::resolve_callees_qualified`) — and S3 makes "shared" true: the four qualifier-discarding traversal helpers and the skeleton/scope path are explicitly brought under the contract (§3.2/§3.6). Slicing goldens drift in the improving direction and are re-blessed with review. |
| Acceptance bar | Directional + no regressions, **with concrete gates** (§5.4). |
| Approach | **Method-owner index + tiered confidence** (over heuristics-only A and full type-confirmed dispatch C, which stays Phase-IP). |
| Confidence storage in CPG | **Deferred to S2** (§7.1). The resolver *returns* confidence; the CPG stores none. **CPG inclusion rule (pinned): Exact + NameOnly edges are included; dropped resolutions are excluded everywhere.** Rationale: measured single-owner method-edge precision is 0.99/0.89 (U-strata) — excluding NameOnly from the CPG would trade little precision for a real slicing-recall regression. Consequence: **Plan B must not ship boundary verdicts before S2 stores confidence**, since NameOnly edges are CPG-visible (§7.1). |
| R6 policy | **P6-lite over P2** (§2.3) — the convergent recommendation of both cleanroom reviewers and the operator scale analysis. |

## 2. The resolution model

### 2.1 The new fact: method owners

`FunctionInfo` (src/ast.rs) gains `owner: Option<String>` and (Go only)
`receiver_var: Option<String>`. Populated in `build_function_table` via an ancestor
walk; per-language owner node kinds live in `src/languages/mod.rs` (new `Language`
method, e.g. `method_owner(node) -> Option<(String, Option<String>)>`).

**OwnerKey normalization:** the owner key is the **bare type name** — generics
stripped (`impl<T> Foo<T>` → `Foo`), pointers/references stripped (Go `*T` → `T`),
no module/package prefix. Known limitation, stated: two same-named types in
different modules share a key, so their methods pool under one owner (affects
single-vs-multi-owner counting in rare cases); module-qualified keys arrive with
S2 span identity / Phase-IP.

| Language | Owner source |
|---|---|
| Rust | Enclosing `impl_item` type. `impl Trait for Type`: indexed under **both** `Type` and `Trait` (dual-key); trait default methods (`trait_item`) → owner = trait name. Trait-key lookups returning **>1 candidate demote to NameOnly** (`kind: trait_cha`) — CHA fan-out must not be labeled Exact. |
| Go | `method_declaration` receiver type + receiver variable name (for R2). |
| Python / JS / TS / Java | Enclosing class, **only if the function is a direct member** — a nested `def` inside a method gets `None` (keeps the descoped nested-def-attribution problem out of the index). |
| C++ | In-class definitions + out-of-line qualified declarators → prefix as owner key, treated **uniformly** (no namespace-vs-class distinction is attempted: `ns::f` indexes under `ns` like `Foo::bar` under `Foo`; R1 lookups work either way, misses fall to the stem rung). |
| Lua | `function M.f()` / `M:f()` → owner `M`. **Explicit keying change:** `FunctionInfo.name` for these goes from `"M.f"` (today's whole-dot-expression capture) to `"f"` with owner `"M"` — a functions-map key change with expected golden drift, called out for re-bless review. |
| C / Bash / Terraform | Always `None`. |

JS object-literal methods (`const obj = {m(){}}`) are `None` in v1. Parse-degraded
files degrade to `None`; existing parse-warning machinery covers this.

CallGraph Phase 1 builds `methods: BTreeMap<(String, String), Vec<FunctionId>>`
keyed by `(owner, name)`, alongside the existing name map, plus an
**owner-by-`FunctionId` side map** (R2 needs the enclosing function's owner;
`FunctionId` itself is unchanged — identity work is S2's).

### 2.2 The call-target contract and the rule ladder

**Parsed `CallTarget` contract (new — extraction today yields only a final name plus
raw qualifier text, and Rust extracts *no* imports, so `Type::m()` currently arrives
as `callee_name="Type::m", qualifier=None`).** Call-site extraction is amended to
classify each site into one of these shapes before resolution:

`T::m` / `mod::T::m` / `mod::f` (Rust/C++ `::` paths, with reserved heads `crate`,
`self`, `super` consumed; `Self::f` resolves the self-type from the enclosing impl) ·
`pkg.f` (qualifier ∈ imports map) · `Class.m` (qualifier = an OwnerKey) · `x.m`
(receiver call) · `self.m`/`this.m`/Go-receiver-var · bare `f()`.

**Invariant:** `qualifier: None` means *verified receiver-less*. Phase-3 synthesized
call sites (fptr/dispatch targets, `call_graph.rs:325-534`) satisfy this today —
the invariant makes it a contract, not luck.

`resolve_callees_qualified` gains the caller's `FunctionId` (for R2/R4b) and returns
`Vec<(&FunctionId, ResolutionConfidence)>`, `ResolutionConfidence = Exact | NameOnly`.

| # | Call shape | Resolution | Confidence |
|---|---|---|---|
| R1 | Qualified `T::m`, `mod::T::m` | Owner-index `(T, m)`; module segments narrow by file when they resolve. | **Exact** (demoted per §2.1 if trait-key >1) |
| R2 | `self.m()` / `Self::f()` / `this.m()` / Go receiver-var | Enclosing function's owner (via side map) → `(owner, m)`; works across split impls (index is repo-wide). | **Exact** |
| R3 | `pkg.f()`, qualifier ∈ imports map | Import narrowing as today, **minus the fall-through**: if narrowing finds no in-repo candidate, return **unresolved** — the qualifier provably names another package (kills the ≈148 caddy `zap.Error`/`caddyhttp.Error` class). Go import matching by package directory/path suffix, not file stem. | **Exact** |
| R3b | `Class.m()` — qualifier is itself an OwnerKey | Owner-index `(qualifier, m)` (covers `ClassName.method()`, Lua `M.f()`/`M:f()`, JS/Java statics). | **Exact** |
| R4 | Unqualified `f()`, caller's file defines `f` as a free function | That definition alone (local-definition preference). | **Exact** |
| R4b | Unqualified `f()` inside a method of owner `K` — **Java/C++ only** (implicit `this`; Java has no free functions at all) | `(K, f)` lookup; falls through to R5 on miss. | **Exact** |
| R5 | Unqualified `f()`, cross-file | Free-function candidates only (methods excluded — sound for Rust/Go/Python where methods require explicit receivers; R4b carves out the implicit-`this` languages). Single candidate → Exact; multiple → all kept, demoted. | **Exact** / **NameOnly** |
| R6 | Receiver call `x.m()`, receiver not covered above | **Three-step policy, §2.3.** Method candidates only, never free functions. | per §2.3 |
| R7 | `mod::f` unresolved by R1 (module-qualified, non-type) | Stem-match fallback, **moved from nav into the shared ladder** as the explicit last rung (ends the two-resolver divergence the substrate analysis flagged as F11; `call_resolve.rs` keeps no private logic). Single stem match → Exact; multiple → NameOnly. | **Exact** / **NameOnly** |

### 2.3 R6 — the three-step policy (cleanroom-convergent)

> "Recover what tree-sitter can prove, demote the single-owner residue, drop the
> rest." Both cleanroom reviewers recommended exactly this shape independently.

1. **P6-lite syntactic receiver-type recovery — Rust + Go only** (the
   confident-change languages), minimal subset:
   - typed parameters (`fn f(x: &mut Foo)`, `func f(x *Foo)`) with `&`/`&mut`/`*`
     stripping;
   - constructor locals: `let x = Type::new(...)` / `let x: Type = ...` /
     `x := Type{...}` / `x := NewType(...)` (the `NewX`→`X` Go convention only when
     `X` is an OwnerKey);
   - `&dyn Trait` / `impl Trait` parameters → trait dual-key lookup (demoted
     `trait_cha` if >1 impl, per §2.1);
   - **shadowing bail:** any rebinding of `x` (`let x`, `x :=`) between the binding
     and the call falls through to step 3 — recovery must be provable, not probable.
   Recovered type **in** the owner index → **Exact**. Recovered type **not in** the
   index (e.g. `Vec`) → **drop**: the receiver is provably external (kills part of
   the stdlib-binding residue P2 alone retains).
2. *(absorbed into the ladder as R3b — qualifier-as-owner; listed here because the
   reviewers framed it as R6 step 1.)*
3. **Residue policy (P2):** exactly one in-repo owner defines `m` → keep, **NameOnly**
   (`kind: r6_single_owner`); multiple owners → **drop** — unresolved, surfaced per
   §2.5. Caller's-own-file preference applies before the repo-wide count (single
   owner defining `m` in the caller's file → resolve there, NameOnly).

**Fallback if P6-lite overruns its budget** (reviewer-recommended): ship step 3
alone (meets the acceptance gates as written), land P6-lite as **S3.1 before
re-baselining**, so acceptance is measured once.

How this addresses the measured classes: tokio `poll`/`as_fd`/`write` are
multi-owner → dropped where untypeable, recovered Exact where the receiver is a
typed param/local (a large share of tokio's 50%-multi-owner sites); caddy
receiver-var `Error` sites → R2 or dropped; caddy package-qualified `Error` → R3
unresolved; `Vec::truncate`→`AccessPath::truncate` → dropped by P6-lite external-type
recovery where typeable, else kept demoted and counted in the named residue.

### 2.4 Known recall cost, stated honestly

Go interface dispatch and Rust trait-object calls with *unrecoverable* receivers and
multiple impls are dropped. The baseline showed some such edges are true (the caddy
interface-dispatch record — note it survives when the concrete impl is the lone
candidate). The corpus rerun measures the net (gate: §5.4); Phase-IP type-confirmed
dispatch (E12 `DispatchProvider`) is the recovery path. P5 (relation-grouped CHA
keep) was evaluated and rejected for S3: the tokio class *is* trait-related fan-out,
so P5 reproduces P≈0.00 by design — it is Phase-IP's job with type confirmation.

### 2.5 Drop visibility (both directions)

- **Callees direction:** dropped/unresolved sites surface through the existing
  unresolved-item representation (`symbol: None`).
- **Callers direction (new):** a callers/ego query whose seed name has same-name
  dropped R6 sites emits a `WarningKind::Collision` (the variant exists,
  `src/navigation/types.rs:86`) carrying the count — an agent asking "who calls
  `MyType::poll`?" sees that N untypeable `poll()` sites exist. Absent edges are
  visible, not silent.

## 3. Components and data flow

1. **`src/ast.rs`** — `FunctionInfo.owner` + `receiver_var`; `CallTarget`
   classification in extraction. **`src/languages/mod.rs`** — per-language
   `method_owner`; P6-lite typed-param/constructor-local readers (Rust, Go).
2. **`src/call_graph.rs`** — Phase 1 `methods` index + owner-by-`FunctionId` side
   map; `resolve_callees_qualified(callee, caller_fid, target)` becomes the
   R1–R7 ladder. **The four qualifier-discarding traversal helpers
   (`callers_of_in_file` :857, `resolve_callers` :883, `callees_of` :925,
   `dfs_cycles` :992) are threaded with each site's qualifier/`CallTarget`** —
   without this, every method call on those paths looks unqualified and R5 would
   strip method edges from barrier/spiral/circular/vertical/3D traversals.
3. **`src/cpg/build.rs` Step 5/5b** — same resolver; **includes Exact + NameOnly,
   excludes drops** (§1). Step 5b arg→param binding rule (pinned): the receiver
   never binds to a parameter in S3; Python explicit-arg binding skips a leading
   `self`/`cls` parameter; per-language tests.
4. **Skeleton/scope path** — `build_skeleton` extracts no qualifiers and
   `compute_scope` resolves through it (`src/cpg/context.rs:356`). Pinned: scope
   computation uses a **recall-biased name-only mode** (current behavior, methods
   included) — scope is a superset heuristic, not a truth claim; the precision
   ladder applies to real edge creation only.
5. **`src/navigation/`** — `call_resolve.rs`'s private stem logic moves into the
   shared ladder (R7). `queries.rs` / `module_graph.rs` map confidence → `score`:
   **Exact = 1.0, NameOnly = 0.6** (0.6 avoids colliding with hop-decayed Exact at
   0.5) × existing hop decay; `module-deps`/`repo-map` aggregate per file-pair,
   max-score-wins. New additive `Reason::Resolution { kind }` (serde snake_case,
   attaches alongside `Calls`/`CalledBy`), kinds enumerated per rung:
   `qualified_owner, self_receiver, import_qualified, qualifier_owner, local_def,
   implicit_this, free_single, free_multi, typed_param, constructor_local,
   trait_cha, r6_single_owner, stem_single, stem_multi`. Scores stop being
   uniformly 1.0.
6. **Caches** — the serialized payload includes `CallGraph`, so the bump is
   `src/cpg_cache.rs:44` `CACHE_VERSION` 3→4 (not a nav-store version). All
   `CallGraph` constructors/mutators must maintain the new index: `empty`, `build`,
   `build_skeleton`, `build_direct_subset`, `remove_files`, `merge`, cache
   deserialization.

**Data flow:** parse → `FunctionInfo{owner, receiver_var}` → Phase 1 owner index +
side map → Phase 2 call sites with `CallTarget` → R1–R7 with confidence → (a) CPG
edges (Exact + NameOnly, no stored confidence) for the 26 algorithms + diff review;
(b) nav Evidence with tiered scores, resolution reasons, and collision warnings.

**Compat posture:** diff-review output is *expected* to drift where false edges
disappear — goldens re-blessed with review (§5.3). Evidence JSON changes are
additive plus score-value changes on demoted classes. MCP tool schemas unchanged
(MCP output shaping already truncates score-sorted lists, so demoted items clip
first under token pressure — an emergent benefit).

## 4. Edge cases

- `Self::f()` inside a trait default method: owner = trait; `(trait, f)` hits trait
  defaults and (via dual-key) impls — demoted if >1.
- Rust split impls across files: repo-wide `(owner, m)` index resolves R2 cross-file.
- Python `super().m()` and other dynamic receivers: fall to R6 step 3 — noted gap,
  Phase-IP.
- Multiple impl blocks for one type: multiple entries under one key — fine.
- Anonymous functions (`name == None`): never indexed.
- Same-file same-name methods: CPG `func_index` is `(file, name)`-keyed,
  last-writer-wins (`src/cpg/build.rs:197-212`) — **outside S3 guarantees**, fixed
  by S2 span-keyed identity.

## 5. Testing and acceptance

1. **TDD throughout** (subagent execution per workflow preferences): per-language
   owner-extraction unit tests (`tests/ast/`), resolver rule tests R1–R7 with
   confidence assertions, nav score/`Reason`/warning tests.
2. **Fixtures (reviewer-specified set):** multi-owner drop; single-owner demote;
   typed-param recovery; shadowing bail; external-type drop (`Vec::truncate` shape);
   trait dual-key demotion; **Java sibling-call survival** (R4b); Go package-path
   import narrowing (R3). Capability matrix: `type_method_qualified` flips
   known_fail→pass; new Rust+Go collision fixtures certify the R6 policy.
3. **Goldens:** drift re-blessed with review — every drifted line must trace to a
   removed false edge, a newly-resolved qualified call, or the Lua keying change;
   unexplained drift is a regression. `Reason::Resolution` kinds make the audit
   mechanical.
4. **Concrete acceptance gates** (full 5-corpus rerun, human-triggered, before/after
   vs `docs/eval/tier-a/baseline.md`):
   - `type_method_qualified` known_fail→pass; **zero** matrix ok→fail flips.
   - tokio C-method callers corrected FP: 390 → **≤20**.
   - caddy C-name callers corrected FP: 441 → **≤30**.
   - No anchor-corpus (prism, caddy) stratum's corrected P or R drops by **>0.02**.
   - R6 telemetry in the PR: counts of dropped multi-owner sites, demoted
     single-owner edges, P6-lite recoveries; the demoted-residue classes named.
   - Flip-candidates pasted into the PR; `baseline.md` updated deliberately after
     acceptance; stale line-keyed adjudications per the dual-adjudicator protocol.
5. **Dev-loop protocol** (CLAUDE.md): `cargo build --release` + `--matrix-only`
   pre-commit on every resolver-touching commit; `--quick` before review.
6. **Perf guard:** owner index + ancestor walk + P6-lite local scans — no measurable
   build-time regression expected; P1/P2 timing protocol if numbers look off.

## 6. Risks and mitigations

| Risk | Mitigation |
|---|---|
| Over-tightening creates false negatives | Tiered fix; recall measured by the same rerun (gate: −0.02 bound, two-sided). |
| **P6-lite wrong recovery ⇒ confidently-wrong Exact edge** | Shadowing bail (provable-only recovery); dedicated shadow fixture; recoveries counted in PR telemetry. |
| Multi-owner drop costs true dispatch edges in C-strata callee recall | Detector = full rerun gates; recovery path = Phase-IP dispatch (§7.3); P6-lite shrinks the exposed class first. |
| Golden churn hides a real regression | Re-bless discipline (§5.3) + mechanical `Resolution{kind}` audit. |
| Owner extraction wrong in a language → silent misclassification | Per-language TDD fixtures first; parse-degraded files degrade to `None`. |
| Score change surprises Evidence consumers | Only demoted classes move off 1.0; `Reason::Resolution` names why; MCP schemas unchanged. |

## 7. Deferred / follow-ups

1. **Confidence on CPG call edges → S2** (owner-approved deferral): store
   `ResolutionConfidence` on `CpgEdge::Call` during S2's batched type churn + cache
   bump; first consumers are Plan B boundary honesty and `gradient_slice`.
   **Sequencing note (binding):** because NameOnly edges are CPG-included (§1),
   Plan B must not ship boundary verdicts before S2 lands.
2. Work-list items 3 (constructor edges), 4 (decl→impl seed mapping, harness-side),
   5 (nested-def attribution).
3. Type-confirmed dispatch via E12 `DispatchProvider` (Phase-IP) — recovers the
   dropped multi-owner residue properly (P5 reaffirmed as Phase-IP, not S3).
4. P6 expansion: typed locals beyond the minimal subset; C++ receivers via
   `type_db`; JS object-literal method owners; Python `super()`.
5. Matrix v2 collision-rich fixtures beyond the Rust/Go sliver (tier-a-followups 8).
6. If P6-lite is deferred at the §2.3 fallback: **S3.1 lands it before
   re-baselining.**
