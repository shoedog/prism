# Python Decorated-Method Double-Capture — Wrapper-Canonical Extraction — Design (2026-06-23)

> The next Python precision slice after self-receiver same-class narrowing (PR #131, this branch's base).
> Basis: the codex xhigh architect analysis (2026-06-23) — this spec formalizes its recommendation
> (Option A, wrapper-canonical) into the design-of-record. **Stacks on #131** (the decorated fix touches the
> same `CallGraph` build paths that #131's `method_class_span` lives in; branch `decorated-double-capture`
> is off the self-receiver HEAD and merges after #131).

## 1. Problem (verified)

prism captures every **decorated Python** function/method as **TWO** `FunctionId`s — the outer
`decorated_definition` (wrapper) AND the inner `function_definition` — with the same logical name and owner
but distinct line spans. Consequence: **every call to a decorated method/function** resolves to ≥2 same
candidates and **demotes to NameOnly** (or, for free functions, mints duplicate Exact edges).

**Verification (codex, with evidence):**
- The functions query captures both node kinds as `@func`: `src/queries.rs:90-97`.
- `ParsedFile::all_functions_via_tree` pushes every capture with **no de-dup** (`src/ast.rs:318-337`);
  `build_function_table` turns each into a `FunctionInfo` (`src/ast.rs:346-370`).
- They normalize to the same name/owner — `Language::function_name` maps a `decorated_definition` to the
  inner name (`src/languages/mod.rs:907-918`); `Language::method_owner` normalizes the inner up to its
  `decorated_definition` parent before finding the class (`src/languages/mod.rs:1084-1099`) — but
  `FunctionId` identity is `(file, name, start_line, end_line)` (`src/call_graph.rs:20-25`), so wrapper and
  inner are **distinct ids**.
- The call graph indexes both into `functions`/`methods`/`method_owners` in all three builders
  (`src/call_graph.rs:491-548`, `:276-303`, `:1478-1507`).
- Live evidence: `eval/fixtures/python/decorator_wrapped/app.py` (`@functools.cache` line 3, `def handler`
  line 4) → `nav_nodes_at` reports two `handler` Function items (span `3-5` and `4-5`); `nav_callees(run)`
  reports BOTH as callees for the single `handler(1)` call.

**Python-only.** JS/TS do **not** double-capture: their grammar nests decorators inside
`method_definition` (no separate wrapper node); prism's JS/TS queries capture the inner method/arrow only
(`src/queries.rs:98-115`). So this slice is **Python-only**; JS/TS get guard fixtures, no behavior change.

## 2. Impact

A **precision** bug (not primarily recall — both physical edges usually survive; the damage is confidence
downgrade + duplicate graph nodes/edges + double body-scan), with broad reach:
- **`self`/`this`/`cls`** (post-#131 same-class narrowing still demotes when `same_class.len() > 1`):
  `src/resolution.rs` self arm.
- **`Cls.helper()` / qualifier-owner** and **typed-receiver R6** route through `owner_lookup`, which demotes
  a >1 same-owner pool — decorated methods lose Exact there too.
- **Free decorated functions** are worse: local free calls can become **multiple Exact `LocalDef` edges**
  (no singleton check on the local-free path) — a duplicate-Exact precision bug, not just a demotion.
- **Caller side:** call-site extraction iterates `all_functions()` again (`src/call_graph.rs:582-643`) and
  scans the whole function byte range, so a decorated body is scanned **twice** → duplicate outgoing caller
  identities.

**Size:** ~418 of ~2,066 pydantic class methods (~20%) are decorated (`@property`/`@*validator`/
`@staticmethod`/…); it affects ALL calls to them, not only self-calls. This is the residual that slice 1a
(self same-class narrowing) explicitly could not fix — plausibly a **larger precision lever than 1a**.

## 3. Goal

**One canonical `FunctionId` per decorated Python definition (wrapper-canonical):** at extraction, keep the
`decorated_definition` node as the single logical function and **skip its inner `function_definition`**.
This removes the duplicate id, the duplicate CPG node, and the double body-scan; preserves decorator-line
ownership; and aligns with the consumers that already treat the wrapper as the entry (framework detection,
scope-honesty).

## 4. Mechanism

### 4.1 Canonicalize at extraction (the core)
In `ParsedFile` function collection (`all_functions_via_tree` / `build_function_table`,
`src/ast.rs:318-370`): when a captured `function_definition`'s **parent is a `decorated_definition`**, skip
the inner node (the wrapper is already captured and carries the same name via `function_name`). Do **not**
collapse arbitrary same-`(owner,name)` definitions — `@overload` stubs, property getter/setter pairs, and
intentional redefinitions are distinct functions and must stay distinct (§7).

### 4.2 Companion: parameter/signature unwrap (REQUIRED — else DFG breaks)
With the wrapper as canonical, parameter/signature helpers must unwrap it. `find_parameters_node`
(`src/ast.rs:3920-3931`) currently only checks direct `parameters`/`declarator` fields → a
`decorated_definition` yields **no parameters**. Add: if the node is a `decorated_definition`, descend to
its inner `function_definition` before locating `parameters`. Audit sibling signature/body helpers
(body-range, return-type, receiver) for the same unwrap need. **Without this, decorated-function params
vanish from the DFG and call-boundary arg→param edges** — a recall regression that must not ship.

## 5. Blast radius (consumers — verify each still behaves)
- `ParsedFile` function table / `all_functions` (`src/ast.rs:279-370`).
- `CallGraph` indexes: `functions`/`methods`/`method_owners`/`method_class_span`/`calls`/`callers`
  (`src/call_graph.rs:143-168`, `:491-548`, `:582-643`).
- Resolution + nav scoring (`src/resolution.rs:455-479`, `:691-780`; `src/navigation/queries.rs`).
- CPG function nodes + call/return edges (`src/cpg/build.rs`); DFG arg↔param (`src/data_flow.rs`).
- Nav seeds / `nodes_at` (`src/navigation/seed.rs`, `queries.rs`) — `nodes_at(def-line)` for a decorated
  def now returns one Function item, anchored at the decorator line (see §8 risk).
- **Already wrapper-aware (must keep working):** inventory has a local Python-wrapper de-dup
  (`src/navigation/inventory.rs:34-56` + `tests/navigation/inventory_test.rs`); FastAPI/Flask framework
  detection walk to the decorator wrapper (`src/frameworks/python/fastapi.rs`, `flask.rs`); scope-honesty
  treats the wrapper as the entry root and filters inner decorated functions
  (`src/reasoning/scope_honesty.rs:176-194`, `:351-370`). Wrapper-canonical is *consistent* with these — but
  test them.
- Cache: `CACHE_VERSION` (`src/cpg_cache.rs`) — bump (extraction/index shape changes).

## 6. Scope

**In:** the wrapper-canonical skip at extraction; the parameter/signature unwrap companion; `CACHE_VERSION`
bump; tests; verify the wrapper-aware consumers (§5) still pass.

**Out (explicit):**
- **Decorator semantics** — do NOT model `@property`/`@classmethod`/`@staticmethod`/pydantic validators/
  overload dispatch/MRO. This slice fixes *logical definition identity* only.
- **JS/TS** — no behavior change (they don't double-capture); add guard fixtures only.
- **The resolution-collapse band-aid** (Option B) — rejected: it leaves duplicate CPG nodes, the double
  body-scan, ambiguous nav seeds, and the free-fn duplicate-Exact bug unfixed, and forces every resolver
  path to re-remember the rule.
- **A-inner** (keep the inner, drop the wrapper) — rejected: loses decorator-line identity and breaks the
  scope-honesty/framework consumers that expect the wrapper entry.

## 7. Soundness
Skip the inner **only** when its parent is a `decorated_definition` (i.e. genuine wrapper+inner of the SAME
function). This is a structural parent-child relationship, not a name match — so `@overload` stubs, getter/
setter pairs, and same-name redefinitions (each its own `decorated_definition` or bare def) remain distinct
`FunctionId`s. No recall loss: the wrapper carries the same name/owner the inner did, so every edge the
inner participated in is reproduced by the wrapper (once the param unwrap is in place).

## 8. Acceptance
- **pydantic:** `kind_exact` rises and `kind_nameonly` falls for `self_receiver`, `qualifier_owner`, and
  typed-receiver buckets; **`multi_target_exact_sites` for decorated free functions DROPS** (the duplicate
  Exact edges collapse); report the deltas.
- **Function-count check:** a decorated top-level function and a decorated class method each produce
  **exactly one** `FunctionId` (and one CPG function node); a decorated function body's calls appear once.
- **DFG check:** a decorated function's params still bind (arg→param edges intact) — guards the §4.2 unwrap.
- **Wrapper-aware consumers:** FastAPI/Flask detection + scope-honesty decorator tests + inventory dedup
  test still pass.
- **Rust/Go** call-stats **byte-identical** (Python-only change); **JS/TS canaries flat**.
- **Tier-A:** `--matrix-only` 0-regr AND, because this touches `ast`/`call_graph`/`cpg`, **`--quick`** before
  review (per AGENTS.md).
- Suite green; `cargo fmt --check` clean.

## 9. Risks
- **Nav start-line shift:** a decorated def's canonical start line moves from `def` to the decorator line.
  Believed correct (the decorator IS part of the function), but expect **snapshot churn** — regenerate +
  eyeball nav/output snapshots.
- **Param-unwrap coverage:** the §4.2 companion is the soundness-critical part; an unaudited signature/body
  helper that doesn't unwrap silently drops decorated-function structure. Enumerate the helpers; test DFG.
- **Decorators change runtime semantics** — out of scope by design; don't let acceptance over-reach.

## 10. Pipeline
Spec (this doc) → codex xhigh spec-review (fold) → writing-plans → codex xhigh plan-review (fold) →
**codex-implement** (per [[feedback_workflow_preferences]]; the orchestrator commits per-task) → host
acceptance (§8) → final codex xhigh diff-review → PR (owner-gated, stacked on #131). Branch
`decorated-double-capture` off the self-receiver HEAD; rebase onto `main` once #131 merges.
