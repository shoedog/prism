# Python Decorated-Method Double-Capture — Wrapper-Canonical Extraction — Design (2026-06-23, rev 2)

> The next Python precision slice after self-receiver same-class narrowing (#131, merged). Basis: the codex
> xhigh architect analysis + spec-review. **Stacks on #131** (branch `decorated-double-capture`, rebased
> onto merged `main`).
>
> **Rev 2 — codex spec-review fold (SHIP-WITH-FIXES; core design holds, completeness fixes):** (BLOCKER) the
> param-unwrap companion was incomplete — **centralize** canonicalization at one point and add a single
> `unwrap_decorated` helper used by EVERY function-node-consuming helper (params, body, statements,
> statement-spans, return-values). (MAJOR) NOT "Python-only" — **C++ templates have the same wrapper/inner
> capture shape** (`template_declaration` + `function_definition`); reworded to "Python decorator wrapper",
> C++ template canonicalization explicitly **deferred** + a C++ **no-change canary** added. (MAJOR) the
> nav **inventory** currently keeps the *inner* and drops the wrapper — wrapper-canonical **inverts** that;
> contract decided + test updated. (MAJOR) the **manual fallback** collector reintroduces the duplicate —
> covered by centralizing before `FunctionInfo`. (MINOR) start-line churn + acceptance test list expanded.

## 1. Problem (verified)

prism captures every **decorated Python** function/method as **TWO** `FunctionId`s — the outer
`decorated_definition` (wrapper) AND the inner `function_definition` — same logical name/owner, distinct
spans. So **every call to a decorated method/function** resolves to ≥2 candidates → **NameOnly** (methods)
or **duplicate Exact `LocalDef` edges** (free functions).

**Evidence:** Functions query captures both (`src/queries.rs:92-96`); `all_functions_via_tree` pushes every
capture with no de-dup (`src/ast.rs:318-337`); `build_function_table` → `FunctionInfo` per node
(`src/ast.rs:347-370`); `function_name`/`method_owner` normalize wrapper→inner name/owner
(`src/languages/mod.rs:907-918`, `:1084-1099`) but `FunctionId` is `(file,name,start_line,end_line)`
(`src/call_graph.rs:20-25`) → distinct ids; both indexed in all 3 builders (`src/call_graph.rs:504-553`,
`:276-303`, `:1478-1507`). Live: `eval/fixtures/python/decorator_wrapped/app.py` → two `handler` Function
nodes, both reported as callees for one call.

### 1.1 Scope of the capture shape (rev-2 correction)
- **Python decorator wrapper: IN scope** (this slice). The `decorated_definition`+`function_definition`
  pair is the target.
- **JS/TS: not affected** — decorators nest inside `method_definition`; queries capture the inner only
  (`src/queries.rs:98-115`). No behavior change; add a guard fixture.
- **C++ templates: SAME shape, OUT of scope (deferred).** `template_declaration` + `function_definition`
  are both captured (`src/queries.rs:129-133`); `template_declaration` is a function node
  (`src/languages/mod.rs:105`) with wrapper name/owner normalization (`:921-929`, `:1127-1135`). This is a
  *separate* double-capture; this slice must **not** change C++ (add a **no-change canary**), and a later
  slice can apply the same canonicalization to C++ templates.

## 2. Impact
Precision bug, broad reach (`self`/`this`/`cls` post-#131 still demotes on `len>1`; `Cls.helper()` /
qualifier-owner + typed-receiver R6 via `owner_lookup` demote; **free decorated functions → multiple Exact
`LocalDef` edges** — the local-free arm returns all local candidates with no singleton guard,
`src/resolution.rs:1238-1255`). Caller side: the decorated body is scanned twice (`all_functions` reused at
`src/call_graph.rs:~582-643`) → duplicate outgoing caller identities. Size: ~418/2066 (~20%) pydantic class
methods decorated; affects ALL their calls. This is the residual slice 1a could not fix.

## 3. Goal
**One canonical `FunctionId` per decorated Python definition (wrapper-canonical):** keep the
`decorated_definition` wrapper as the single logical function; **drop the inner `function_definition`**.
Removes the duplicate id / CPG node / double body-scan; preserves decorator-line ownership; aligns with the
consumers that already treat the wrapper as the entry (`scope_honesty` entry-root, framework detection).

## 4. Mechanism

### 4.1 Centralize canonicalization (covers BOTH extraction paths — BLOCKER fold)
Apply ONE canonical filter before `FunctionInfo` records are built, so **both** the query path
(`all_functions_via_tree`, `src/ast.rs:318-337`) and the **manual fallback** (`collect_functions_manual`,
`src/ast.rs:466-474`, reachable via the reconstruction fallback `src/ast.rs:286-288`) drop the inner: for a
captured `function_definition` whose **parent is a `decorated_definition`** (Python only), skip it (the
wrapper is already captured and carries the same name via `function_name`). Do NOT collapse arbitrary
same-`(owner,name)` defs — `@overload` stubs, getter/setter pairs, redefinitions are distinct and stay
distinct (the predicate is structural parent-child, matching the trusted `scope_honesty.rs:364-370`
discriminator). Centralizing (vs filtering each path) prevents the fallback from reintroducing the dup.

### 4.2 `unwrap_decorated` helper, used by ALL function-node helpers (BLOCKER fold)
With the wrapper canonical, every helper that takes a function node and reads a child field must first
unwrap a `decorated_definition` to its inner `function_definition`. Add one helper
`unwrap_decorated(node) -> node` and call it at the head of **each** of:
- `find_parameters_node` (`src/ast.rs:3922-3931`) — params (else DFG/arg→param edges vanish).
- `function_body_node` (`src/ast.rs:2607-2611`) — CFG/body.
- `statements_in_function` (`src/ast.rs:3097-3104`) and `statement_spans_in_function` (`:3112-3115`).
- `return_value_nodes` (`src/ast.rs:2828-2893`) — **incl. the nested-function guard at `:2888-2893`** which
  otherwise drops the inner body's returns.
Audit siblings (receiver/signature/name-occurrence helpers) for the same field-access pattern; any that
descend by field need the unwrap. **This audit is the soundness-critical part — a missed helper silently
drops decorated-function structure (recall regression).**

## 5. Blast radius (verify each)
- Extraction: `ParsedFile` table + both collection paths (§4.1).
- `CallGraph` indexes / resolution / CPG nodes+edges / DFG (`src/call_graph.rs`, `src/resolution.rs`,
  `src/cpg/build.rs`, `src/data_flow.rs`).
- **Inventory contract FLIP (MAJOR):** `src/navigation/inventory.rs:34-56` currently de-dups by **keeping
  the inner and dropping the wrapper**. Wrapper-canonical removes the inner, so inventory must keep the
  **wrapper** — update the local de-dup (it may become a no-op or invert) and its test
  (`tests/navigation/inventory_test.rs`); note the start-line/kind churn.
- Nav `nodes_at` / seeds: `nodes_at(def-line)` becomes enclosing-evidence; the exact function node moves to
  the decorator line (CPG indexes at `fid.start_line`, `src/cpg/build.rs:342-358`).
- Algorithm consumers of params/returns: contract postconditions (`src/algorithms/contract_slice.rs:909`),
  reasoning seeds (`src/reasoning/seeds.rs:185-203`) — covered IF §4.2 is complete; test them.
- Wrapper-aware (must keep working): `scope_honesty.rs:176-194,:351-370`; FastAPI/Flask detection
  (`src/frameworks/python/`).
- Cache: `CACHE_VERSION` bump.

## 6. Scope
**In:** centralized wrapper-canonical skip (both paths); the `unwrap_decorated` helper + the full helper
audit; inventory contract update; `CACHE_VERSION` bump; tests.
**Out:** C++ template canonicalization (separate slice — add only a no-change canary here); JS/TS (no
change, guard fixture); decorator semantics (`@property`/`@classmethod`/MRO/validators); the
resolution-collapse band-aid (rejected); A-inner keep-inner (rejected — breaks scope-honesty/framework).

## 7. Soundness
Skip the inner ONLY when its parent is `decorated_definition` (structural, not name-based) → `@overload`/
setters/redefinitions stay distinct. The wrapper carries the inner's name/owner, so every edge is
reproduced **once §4.2 is complete**. No recall loss conditional on the helper audit (the explicit risk).

## 8. Acceptance
- **pydantic:** `kind_exact` rises / `kind_nameonly` falls for `self_receiver` + `qualifier_owner` +
  typed-receiver; **`multi_target_exact_sites` for decorated free functions DROPS** (duplicate Exact
  collapse); report deltas; overall canary not increased.
- **Function-count:** a decorated top-level fn and a decorated method each → exactly one `FunctionId` + one
  CPG node; decorated body calls appear once.
- **Helper tests (guards §4.2):** for a decorated function — `function_parameter_names`/occurrences,
  `function_body_node`/`statements_in_function`/`statement_spans_in_function`, `return_value_nodes` all
  return the inner's content (not empty); DFG arg→param intact.
- **Inventory:** decorated fn appears once, anchored at the wrapper (start/kind churn captured).
- **Free-fn:** a decorated local free call resolves to exactly ONE Exact `LocalDef` (was two).
- **C++ no-change canary:** a C++ template function's call resolution + function count **unchanged**.
- **Wrapper-aware:** FastAPI/Flask + scope-honesty decorator tests pass.
- **Rust/Go** call-stats byte-identical; **JS/TS** canaries flat.
- **Tier-A:** `--matrix-only` 0-regr; touches `ast`/`call_graph`/`cpg` → run `--quick` before the diff-review
  too (best-effort; Rust/Go byte-identical is the primary inertness proof).
- Suite green; `cargo fmt --check` clean.

## 9. Risks
- **Helper-audit completeness** (§4.2) is the soundness-critical risk — enumerate + test every function-node
  helper; a miss = silent structure loss. Centralization (§4.1) handles the extraction side; the helpers are
  the read side.
- **Nav start-line shift** `def`→decorator — snapshot churn; regenerate + eyeball.
- **C++ inertness** — the canonical filter is Python-gated; the C++ canary guards it.
- Decorators' runtime semantics — out of scope.

## 10. Pipeline
Spec (rev 2) → codex spec re-review → writing-plans → codex plan-review → codex-implement → acceptance (§8)
→ final codex diff-review → PR (owner-authorized; merge on green CI). Branch `decorated-double-capture`.
