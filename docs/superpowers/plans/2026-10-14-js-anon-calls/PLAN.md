# Lane A2: calls inside anonymous JS/TS callables. Census, matrix and plan

**Status:** step 0, plan only. No product code. Branch `plan/js-anon-calls`, base main `b593ad18`.
**Decision it serves:** the owner chose shape B with fallback A (2026-10-14). This plan gives the numbers, the identity and naming design, the fallback-A list and the slice split. The owner approves them before any implementation starts.
**Evidence:** `~/prism-evidence/js-anon/`. It holds the base and prototype `sites.jsonl` for X and T, `census/{X,T}/aggregate.json` and `samples.jsonl`, `timing/`, `matrix-{base,proto}.log`, `caller-check-{base,proto}.jsonl`, and `bin/` (both binaries, `BASE-COMMIT`, `prototype.diff`).

## 1. Headline

- **Reproduced.** On main `b593ad18`, the anonymous callables of X (Excalidraw `0642e72c`) contain **3,275** product call nodes with no record, and those of T (TypeScript `src` at `7964e22f`) contain **3,306**. These are the js-arg-follow census numbers. The base `sites.jsonl` is byte-identical to that lane's.
- Counted under prism's own naming rule rather than the TS checker's, the gaps are X **3,268** and T **3,248**. In test code there are another 21,062 (X) and 15,391 (T).
- **What B would record.** The prototype makes each anonymous callable own its calls. It records X **3,139** and T **3,200** of those product calls.

  | Product calls newly recorded | X | T |
  |---|---:|---:|
  | Exact | 1,046 (33%) | 1,808 (57%) |
  | NameOnly | 459 (15%) | 430 (13%) |
  | Dropped (mostly `UnknownName`: external `.map`, `setState`, DOM) | 1,634 | 962 |
  | Still unrecorded: `new Map()`, `new Error()`, and other `new` of a builtin (not specific to anonymous callables) | 129 | 48 |

- **Precision signal.** The pinned checker resolved most of the new Exact targets.
  - Its disagreement rate on the new Exact targets is about the same as on the Exact targets prism already records:

    | Exact targets the checker disagrees with | X | T |
    |---|---:|---:|
    | New (prototype) | 55 / 573 = 9.6% | 148 / 1,808 = 8.2% |
    | Existing (base) | 301 / 5,458 = 5.5% | 3,636 / 31,714 = 11.5% |

  - The disagreements are not adjudicated. The ones spot-checked are checker-line artifacts (overloads) or #382 returned-value targets, the same classes seen in base.
  - **There is one real defect class.** A callback has no locals in prism, so a name bound by the callback's own parameter, or by a local of the enclosing function, is not shadow-guarded. It can mint a false Exact. The census shows 11 X and 3 T `free_single` disagreements of this kind. The matrix confirms the mechanism (4 rows, §4). Slice S2 must fix it.
- **Nodes and edges, product code.**

  | | X | T |
  |---|---:|---:|
  | Anonymous callables (all files) | 1,937 (4,498) | 1,852 (5,292) |
  | … that make calls themselves | 1,502 | 1,562 |
  | … that sit inside an enclosing callable (creates edges) | 1,870 | 1,840 |
  | … at module level | 67 | 12 |
  | `call_site` records, all files (base → prototype) | 19,140 → 44,363 | 61,705 → 81,102 |

- **Cost (prototype, two runs each, `nav --no-cache call-stats`).**
  - Wall time: X 32.1 / 32.1 s → 42.2 / 45.4 s (+31–41%). T 216.6 / 245.2 s → 257.4 / 294.0 s (+19–20%).
  - Max RSS: X 893 / 894 → 969 / 879 MB. T 2.41 / 2.41 → 2.67 / 2.31 GB. That is within run-to-run noise.
  - The time grows with the number of records (2.3× on X, mostly test code). Not profiled. My hypothesis is that `call_capture_owner` reconstructing `all_functions()` for each call is the hot spot (`ast.rs:10049`).
- **Identity is unique.** No two anonymous callables in one file share `<cb@L:C>`: 0 collisions among 4,498 on X and 5,292 on T. Several callables start on one line 12 times on X and 89 times on T; the column separates them.
- **Matrix.**
  - 80 new rows (`step: "anon"`). Today: 64 NO-RECORD, 12 CORRECT (controls), 4 MISSING.
  - Prototype:
    - 60 rows flip to CORRECT.
    - 4 flip to WRONG: the shadow defect.
    - 2 class rows gain a duplicate record.
    - 2 generator rows and 2 IIFE-callee rows do not move.
  - **None of the 1,349 existing rows changes.**
- **Fallback A is already live in one place.** Calls in a callback inside a class method are recorded today under the method, because class bodies keep the legacy call inventory (`ast.rs:10068`, from 8e2f64ff). If B is applied naively there, those calls are recorded twice: 972 X and 1,062 T existing call nodes gain a second caller (§5).

## 2. Ground truth: why the calls are missing

- `CallGraph::build` Phase 2 (`call_graph.rs:1849`) skips any callable for which `Language::function_name` returns `None` (`None => continue`).
- `function_name` (`languages/mod.rs:1157`) names arrows and function expressions by ECMAScript-style inference from the direct parent only:
  - P1 `const X = …`;
  - P2 `{k: …}`;
  - P3 `const X = call(…, () => …)`;
  - P4 a class field;
  - P5 `a.b = …` or `x = …`.
- Everything else is anonymous. That includes a parenthesized `const X = (() => …)`, because tree-sitter keeps the parentheses as a node.
- The named enclosing callable does not pick the calls up either. `call_capture_owner` (`ast.rs:10037`) returns `Nested` for a call whose innermost callable boundary is another callable. The exception is class bodies, which return `LegacyInventory`: the method keeps every call in its byte range.
- PR-B (`ast_callback_identity.rs:199`) already gives these callables a DFG-only owner `<cb@L:C>`: 1-based line and 1-based UTF-8 byte column of the first byte, for `arrow_function` and `function_expression` without a `function_name`.
  - It is persisted through `VarLocation.function` (CPG cache v108+, now 117).
  - Its doc comment states the invariant this lane must retire: the name never reaches the call graph, seeds or navigation.
- `generator_function` (`function* () {}`) is not in PR-B's synthetic set, so anonymous generators get neither a DFG pass nor a node.

## 3. Census detail (product code, prism naming rule)

Script `measure/anon-census.cjs` (static, with the pinned TS 5.9.3 checker). It joins every `call_site` record to its syntax node by UTF-8 byte span, for base and prototype.

**By the role of the innermost anonymous callable**

| Role | X | T |
|---|---:|---:|
| array method callback (`.map`, `.forEach`, `.filter`, …) | 1,056 | 843 |
| JSX prop handler (`onClick={() => …}`) | 597 | 0 |
| argument to an in-repo function | 475 | 2,120 |
| React hook (`useEffect` 291, `useLayoutEffect` 36, `useMemo` 10, `useCallback` 5) | 342 | 0 |
| argument to another function (external) | 254 | 50 |
| returned (`return () => …`, curried) | 235 | 132 |
| promise `.then`/`.catch` | 101 | 0 |
| timer or listener | 65 | 9 |
| `new Promise` executor | 51 | 1 |
| IIFE | 42 | 26 |
| other (conditional, `as`-wrapped, JSX child, default parameter, default export, …) | 50 | 67 |

| Anonymous depth (anonymous callables up to the named one) | X | T |
|---|---:|---:|
| 1 | 2,722 | 2,959 |
| 2 | 535 | 272 |
| 3+ | 11 | 17 |

| What holds the outermost anonymous callable | X | T |
|---|---:|---:|
| a named function | 2,629 | 3,224 |
| a class-field initializer (`handle = debounce(() => …)`) | 318 | 0 |
| module level | 288 | 13 |
| a class method (recorded today only in some layouts) | 33 | 11 |

**Prototype outcome by role (X):**
- array methods: 509 Exact of 1,041;
- JSX props: 104 Exact of 584;
- `useEffect`: 87 of 285;
- in-repo arguments: 171 of 455.

**T:** in-repo arguments 1,255 Exact of 2,100; array methods 423 of 827. (`by_grade_and_kind` in the aggregates.)

**Not this lane.** Product calls with no record outside anonymous callables:
- module-level code: X 1,581, T 353;
- named callables, from `Unsupported` or `Outside` owner decisions: X 570, T 657.

## 4. Matrix (`tests/fixtures/js_alias_matrix/`, `step: "anon"`)

- **Generator.** `generator/variants-anon.cjs` holds 40 shapes × {ts, js}; JSX shapes are written as `.tsx`/`.jsx`. Each row has the normal callee `truth` plus an authored **`caller` truth**:
  - `{kind:"anon", file, line, col}`, computed from a `/*C*/` marker; or
  - `{kind:"named", name}`.
- `export.cjs` writes `caller` only on rows that have one, so the 1,349 existing fixture lines are byte-identical.
- `expected.jsonl` is blessed on base. The new rows are appended; the existing rows are byte-identical (checked with `cmp`).
- The matrix test passes, 23/23 in `js_alias_matrix`.
- **The test does not read `caller` yet.** Slice S2 adds a caller check (a site record's caller must match the truth). `measure/caller-check.cjs` is the dev-time version.
  - On base: 64 NO-RECORD, 12 OK, 4 WRONG (`run` and `A` instead of the callback).
  - On the prototype: 74 OK, 4 OK+extra, 2 NO-RECORD. The authored columns match prism's `<cb@L:C>` on every recorded row.

| Rows (×2 languages) | Today | Prototype | Planned (end of S2) |
|---|---|---|---|
| arrow expression/block body, async arrow, function expression, nested-2, nested-3, nested in a named local, siblings on one line, IIFE arrow/function, argument to repo/external function, `.then`, `new Promise`, JSX prop handler, JSX element in `.map`, `useEffect`, `useEffect` cleanup, unassigned `useCallback`, class-field wrapped, object method callback, object wrapped property, default-parameter arrow, `export default` arrow, module-level callback, parenthesized const, alias outside/inside the callback, returned value in the callback (29 shapes) | NO-RECORD | CORRECT | CORRECT, caller = anon |
| minified one-line | MISSING | CORRECT | CORRECT |
| callback parameter shadows the import; enclosing local shadows a decoy | NO-RECORD | **WRONG** (false Exact) | MAY-NONE |
| class method callback | CORRECT (caller `run`) | CORRECT, duplicate record | CORRECT, caller `run` (fallback A1) |
| anonymous generator expression | NO-RECORD | NO-RECORD | CORRECT (if D3 = include) |
| IIFE callee (`(() => 1)()`, truth = the anonymous callable) | MISSING | MISSING | CORRECT in S3 (invokes edge) |
| controls: named function expression, assigned `useCallback`, assigned `useMemo`, class field arrow, object arrow property | CORRECT | CORRECT | CORRECT |
| `param.inline-arrow-arg.*` (existing, 6 rows) | MISSING | MISSING | A2b (argument following to anonymous targets), not this lane |

#372/#382 still apply inside callbacks. In the prototype, alias-outside, alias-inside and returned-value-in-callback all resolve CORRECT, because neither rule is keyed on the caller's name.

## 5. Identity, naming, and how consumers query it

This section answers the owner's three queries and the controller's proposal.

### 5.1 Key

**Proposal: positional key.**
```
FunctionId { file, name: "<anon@L:C>", start_line, end_line }
```
- `L` is the 1-based line; `C` is the 1-based UTF-8 byte column of the callable's first byte.
- **Unique by construction.** Two distinct callables cannot share a first byte: an arrow or function expression starts with `async`, `(`, an identifier or `function`, and none of those starts another callable. Measured: 0 collisions on X and T.
- **Several on one line** (X 12, T 89, and every minified file): the column separates them.
- **TSX:** same grammar rule.
- **Minified code:** `C` can be large. Fine for a key; a display concern only (5.3).
- **Byte columns, not UTF-16.** `C` matches PR-B and prism's span convention but not LSP/editor columns on a line with non-ASCII text before the callable. The display can add a UTF-16 column if wanted. The key does not need one.
- **Disambiguator:** none needed beyond file + `L:C`. `start_line`/`end_line` are redundant but kept so `FunctionId` keeps its shape.

**Prefix `<anon@` instead of `<cb@`: agree.**
- About half the nodes are not callbacks: X has 597 JSX-prop calls, 235 returned, 42 IIFE, default exports. T has 132 returned.
- **Rename cost, measured with grep:**
  - one constant (`SYNTHETIC_OWNER_PREFIX`);
  - 127 literals in `src/cpg/callback_identity_tests.rs`;
  - 24 in `src/cpg/fixtures/js_param_r10_controls.json`;
  - 2 in `js_param_defs_tests.rs`;
  - doc comments in `ast_callback_identity.rs`, `data_flow.rs`, `navigation/queries.rs:2034` and `cpg_cache.rs`;
  - one MCP-visible sentence in `docs/MCP.md:130`;
  - a CPG cache bump, because persisted DFG rows carry the name.
- It is mechanical (sed plus a review) and **wire-visible**. Today `<cb@…>` appears in `taint_reaches` `Variable.function` and in MCP Evidence variable descriptions. Old plan documents under `2026-10-06-js-param-defs/` keep `<cb@` as history.
- Do it as its own slice, S0, so no behavior change rides with it.

### 5.2 Structured fields (not part of the key)

`CallGraph.anon_meta: BTreeMap<FunctionId, AnonMeta>`, persisted, consumed by nav JSON, Evidence and MCP:
- `anonymous: true`;
- `col`, plus `kind` (`arrow` | `function` | `generator`) and `async`;
- `enclosing: Option<FunctionId>`: the nearest enclosing callable, named or anonymous. The chain is followed through the map. `None` at module level and in class-field initializers; those carry `enclosing_scope: "module" | "class C field handle"`.
- `passed_to: Option<{ callee_text, arg_index, call_line, call_span, callee: Option<FunctionId> }>`, when the callable is an argument. `callee` is filled only when the existing call record resolves Exact. Examples: `useEffect`, arg 0; `each` → `lib.ts:each`.
- `bound_as: Option<…>`, one of:
  - `jsx_prop { element, prop }`;
  - `object_key`;
  - `class_field`;
  - `default_export`;
  - `variable_parenthesized`;
  - `default_param { function, param }`;
  - `returned`;
  - `iife`.
- `label: String`, display only, derived from the fields and never used as a key:
  - `Editor › useEffect callback (4:11)`;
  - `Toolbar › onClick handler (12:20)`;
  - `A › .map callback (3:18)`;
  - `module › forEach callback (1:12)`;
  - `C.handle › debounce callback (3:21)`;
  - nested: `A › .map callback › .map callback (2:53)`.

Every role in §3's table is available syntactically at mint time. The census computed the same roles from the TS AST, and the Rust side has the parent chain.

### 5.3 The three queries

1. **Anonymous functions in file x.js.**
   - `prism nav functions --repo R` already lists the FunctionTable: anonymous entries have `name: null` and lines, but no column and no context.
   - Proposal: extend that inventory, additively, with the 5.2 fields, and add `--file <path>` and `--anonymous` filters.
   - No new query. Tier-A reads the inventory by field name, so additive fields are safe (verify in S3).
2. **Anonymous functions in func().** `nav functions --in <symbol|loc>` filters on the `enclosing` chain (transitive by default; `--direct` for one level).
3. **Where an anonymous function is used and called.**
   - **Created / handed off:** the creates edge (enclosing → anon), and `passed_to` with the call site.
   - **What it calls:** `nav callees --at file:L:C`. Location seeds already exist. S1 makes a location seed on a callback line resolve to the named enclosing function by default; `--at L:C` picks the anonymous node exactly.
   - **Who invokes it:**
     - in-repo invokers come from argument following (A2b, later lane: `JsArgValue::Inline(span)` exists from A1a);
     - external invokers (React, `setTimeout`, Promise) cannot be seen, so the answer is the `passed_to` site, labelled "handed to external `useEffect`";
     - an IIFE is its own invoker: the S3 `invokes` edge, Exact.

   **Where it falls short:**
   - (a) Before A2b, "who calls this callback" stops at the hand-off site for in-repo higher-order functions too.
   - (b) A callback stored in a variable or field and called later is not followed.
   - (c) External invokers are never claimed as edges.

   **Existing queries are not enough on their own.** `callers foo` would list `<anon@4:11>` with no context unless the label and the `enclosing` chain ride along. That is the 5.4 display rule.

### 5.4 Display rule

Nav `callers`/`callees`/`ego` text, JSON, Evidence and `--format callers` show an anonymous node as its label. JSON also carries the raw `FunctionId` and the fields.
- **Collapse rule for `callers`:** when a caller is anonymous, its entry shows the label (which names the named root, e.g. `A › .map callback`). The named root is not removed from depth accounting, so `callers foo --depth 1` shows `A › .map callback (3:18)`.
- That answers the owner's goal: `nav callers foo` now names `A`. The identity key is not touched.

### 5.5 Stability: positional or structural

**Recommend positional, with a structural anchor for matching across versions only.**
- Named `FunctionId`s already carry `start_line`/`end_line`, so any edit above a function changes every key below it today. Positional anonymous keys are no less stable than the status quo.
- Both caches are content-fingerprinted: CPG v117 and nav edge cache v65 (`navigation/call_edge_cache.rs:110`). A shifted key can never be a stale hit; it is a re-derivation.
- **Diff review (base vs head):** `delta_slice` compares DFG edges by `(file, line)` (`delta_slice.rs:41`), already positional.
- **Structural alternative** `(enclosing named FunctionId, ordinal among anonymous callables in preorder, passed_to.callee_text)`:
  - It is stable under edits outside the enclosing function, but every named key it embeds still shifts.
  - It is unstable when an earlier callback is inserted into the same function.
  - It is undefined for module-level and class-field callables without an extra scope token.
  - Cost: a second identity scheme beside PR-B's.
- **Use:** compute it as `anchor` in `AnonMeta`, for any consumer that must match one callable across two versions. No such consumer exists today. Do not make it the key.

## 6. Which callables become nodes, and the edge to them

- **Nodes (B):** every JS/TS/TSX `arrow_function`, `function_expression` and (D3) `generator_function` that `function_name` leaves unnamed. That is exactly where `js_ts_synthetic_callable_name` returns `Some`, extended with generators.
- **Parameter early errors:** callables where `dfg_owner_name` returns `None` because of a parameter early error still get a node. Call ownership does not need the DFG pass; the node simply contains no DFG rows.
- **One helper:** one shared helper names owners for all three builders. These are the full build Phase 1 and Phase 2, `build_skeleton` (`call_graph.rs:1373`/`1439`) and `build_direct_subset` (`5213`/`5285`), plus CPG Step 1 and Step 6. Any disagreement between them drops nodes from the scoped CPG.
- **Ownership:** an anonymous node owns the calls whose innermost callable boundary is itself. This is the existing `Owned`/`Nested` rule, unchanged.
- **Creates edge (enclosing → anon).**
  - It is *not* a `CallSite`; `resolution.rs:64-70` warns that a synthetic CallSite is a soundness hole.
  - It follows the precedent of `RegistrationRecord { enclosing, target }` (`call_graph.rs:719`), which nav merges as an edge.
  - New `AnonCreation { enclosing: Option<FunctionId>, anon: FunctionId, span }`.
  - Nav edge kind `ResolutionKind::AnonCreation`, confidence **Exact**: creation is syntactic containment and is certain.
  - Displayed as "creates", never "calls".
  - Module-level and class-field anonymous nodes have no creates edge (D4).
- **Invokes edge (S3):** for an IIFE, `(anon)()` resolves Exact to the anonymous node, as a real `CallSite` with `pre_resolved_target`. This is the only case where the creator provably calls it.

## 7. Fallback A list

Each entry credits calls to the nearest named enclosing function instead of minting.

| # | Case | Why B is not sound or cheap here | Size (product) | Fallback |
|---|---|---|---|---|
| A1 | Anonymous callable inside a **class method** (owner in class context; `LegacyInventory`) | The method already records every call in its range, including inside callbacks (8e2f64ff kept this on purpose). Minting duplicates the record: 972 X / 1,062 T existing nodes gain a second caller, and 5 X / 2 T change their combined grade. Moving the calls off the method changes existing records, which the "no existing record changes" gate forbids in the first slice. | X 972, T 1,062 (existing records) | Keep today's attribution to the method. Mint no own-call records for these nodes. Revisit in S5 (D2). |
| — | Class **field initializer** (`handle = debounce(() => …)`) | Not a fallback case: there is no named enclosing callable to fall back to, and no method claims these calls today (X 318 unrecorded). | X 318 | B, with strict ownership. The anon must use strict `Owned`/`Nested` even though it is inside a class, or it would also record its nested callbacks' calls. |

That is the only fallback. The other candidates were checked and do not need one:
- Module-level callbacks: no named enclosing callable; B node with `enclosing: None`.
- Default-parameter arrows: enclosing is the function; the census found 2 X / 2 T.
- Nested chains: B handles them.
- Same-line siblings: the column handles them.
- Minified code: the column handles it.
- Parameter early errors: node without DFG.
- Generators: B if D3.

## 8. Consumer changes

From a sweep of every `FunctionId`/caller consumer. The slice that makes each change is in brackets.

- **Seeds and "enclosing function" lookups** [S1]:
  - CPG `function_at` (`cpg/query.rs:394`) and nav `enclosing_function` (`navigation/mod.rs:563`) pick the smallest range, and the anonymous node wins ties. Without a guard they re-seed barrier, membrane, echo, vertical, threed and spiral, plus every nav `loc:` query, onto `<anon@…>`. That includes any line that merely contains a one-line arrow.
  - Membrane would lose the cross-file callers of the outer function. Spiral's test ring searches by function name and would find nothing.
  - **Change:** both skip anonymous nodes by default. An explicit `--at L:C` or anonymous-aware callers opt in.
  - Also reasoning `function_starts_at` (`cpg/trace.rs:628`), so handler-callback seed lines are not degraded to pure taint.
- **Symbol seeds** [S1]: `navigation/seed.rs:58` resolves symbols by exact `name_index` string. Reject the `<anon@` prefix as a symbol; anonymous nodes are reached by location only.
- **Visibility** [S1]: nodes-at (`queries.rs:2036`), ego Contains hops to synthetic Variables, onboarding function counts (`onboarding.rs:86`), and MCP next-query suggestions (`mcp/evidence_view.rs:1052`) must keep hiding synthetic identities, as SPEC-prB D7 required.
- **R4c import-shadow guard** [S2, **must**]:
  - `js_ts_function_locals` is built only for named callers (`call_graph.rs:2245`), and the guard reads it by caller (`resolution.rs:3443`, `3611`).
  - Build locals for anonymous owners, and consult the whole lexical chain (anonymous → … → named).
  - Otherwise callback parameters and enclosing locals mint false Exacts: 4 matrix rows, 14 census `free_single` disagreements.
- **CPG Step 1 bytes** [S2]: `cpg/build.rs:985-1005` matches `FunctionInfo.name == fid.name`, so anonymous nodes get bytes (0,0). Match on byte range and kind instead.
- **Self receivers** [S2, recall]: `this.x()` in an arrow inside a method is dropped (`UnknownName`), because `method_owners` has no anonymous entry. Arrows bind `this` lexically, so inherit the enclosing method's owner for arrows only, not for `function` expressions.
- **Step 5b arguments → parameters and taint** [S2, measure]:
  - Calls inside callbacks gain interprocedural arg→param DFG edges. Taint gains recall and possibly false positives.
  - Run the taint suites, Tier-A `--quick` and `taint_reaches` before/after. Report the deltas; do not re-baseline.
- **call-stats / dump-sites** [S2]: counters grow. The dump shows `caller.name: "<anon@…>"`, and S3 adds `caller.anon`. Count creates edges separately; they are not call sites.
- **`--format callers`** [S3]: `output/review.rs:427` walks `callers` by **bare callee name**. Once anonymous nodes are callees (creates and invokes edges), the bucket `<anon@3:10>` is shared across files and pulls in other files' creators. Key the walk by `FunctionId` or filter it by file, and display labels.
- **Nav callers/callees/ego, MCP Evidence, review/SARIF/targets function names** [S3]: show the label; JSON keeps the raw id and the fields. `targets` falls back to the anonymous name when echo or membrane reports one; map it to the label.
- **Mermaid** [S1]: `output/mermaid.rs:58` escapes only `"`. `<` and `>` need escaping (circular slice labels).
- **Algorithms:**
  - Through `parsed.enclosing_function` (already returns arrows today): parent_function, thin, left/full_flow, relevant, horizontal, symmetry, quantum, absence, contract, angle, primitive, callback_dispatcher, provenance, and taint seeds/sinks. Unchanged.
  - Circular gains cycles through callbacks; intended.
  - Gradient gains a seed; fine.
- **Taint descent** (Exact-gated, `cpg/trace.rs:685-771`): descent from inside callbacks into named callees becomes possible (new recall). An anonymous callee has no parameters, so it is never descended into.
- **#372/#382 alias rule** (`resolution_js_alias.rs`, `resolution_js_returns.rs`, `via_alias_route`, stand-down, callers index): keyed by `caller.file` plus span plus `functions[name]`, not by the caller's name. No change; matrix rows confirm.
- **`--algorithm taint` framework seeds and #376 item 10:**
  - Seeds come from `function_name` on the AST, so this lane does not change them.
  - Express entries (`framework_entries/express.rs:480`) walk past anonymous scopes to the named function, so inline handlers stay "unresolved".
  - The owner-name mismatch in #376/10 is the same shape: the framework record's caller is the named function while the call site's caller becomes `<anon@…>`.
  - Map framework entries to the anonymous node in S3, or leave them; this is decision D6.
- **Tier-A** (`eval/tier_a/compare.py:57`): sites are compared by `(file, line)`, and the caller comes from the inventory, so anonymous caller names are not mismatches. New sites inside callbacks will appear. Exclude creates edges from callee extraction. Re-run `--matrix-only` and `--quick` in S2 and S3, and paste flips into the PR rather than re-baselining.
- **Caches:**
  - S0: CPG 117→118 (rename in persisted DFG rows).
  - S2: CPG →119 and nav edge 65→66 (new nodes and records).
  - S3: CPG →120 and nav edge →67 (`AnonMeta`, creates/invokes edges).

## 9. Slices (each sized to converge in two review rounds)

| Slice | Content | Gate | Matrix rows it flips |
|---|---|---|---|
| **S0** rename | `<cb@` → `<anon@` everywhere PR-B uses it; MCP.md note; CPG bump | No behavior change: whole suite green with only the literal changes; Tier-A matrix-only unchanged | none |
| **S1** guards | Skip-anonymous defaults in `function_at`/`enclosing_function`/`function_starts_at`; reject `<anon@` symbol seeds; visibility filters (nodes-at, ego, onboarding, MCP suggestions); mermaid escaping; one shared owner-name helper (unused for minting yet). Tests build graphs with anonymous nodes directly. | No-op on main: matrix and Tier-A unchanged. Unit tests for each guard on a fixture with callbacks. | none |
| **S2** nodes and records | Mint anonymous nodes in all builders. Own-call records with strict ownership, plus fallback A1 for class methods. Locals for anonymous owners and the chain lookup (R4c). Arrow `this` owner inheritance. CPG Step-1 bytes. Matrix test learns `caller`. Cache bumps. | (1) No existing call record changes on X/T: census regression check, `caller_changed` = 0. (2) 0 of 1,349 existing matrix rows change. (3) Shadow rows are not WRONG. (4) Taint/Tier-A deltas reported. (5) Build time and RSS on X/T reported against the prototype's (+31–41% / +19–20%). | 60 → CORRECT; 4 shadow rows → MAY-NONE; generators if D3 |
| **S3** edges, fields, display | `AnonMeta` (enclosing, passed_to, bound_as, label, anchor); creates edges; IIFE invokes edge; `--format callers` keyed by FunctionId; label display in nav, Evidence, MCP, SARIF, targets; `nav functions --file/--anonymous/--in`; MCP.md and skills docs | Golden nav and MCP output for each `bound_as` kind; Tier-A quick | 2 IIFE-callee rows → CORRECT |
| **S4** (optional) cost | Cache or index `all_functions()` per file for `call_capture_owner`, if S2's measured cost is unacceptable (D5) | Byte-identical output; time measured | none |
| **S5** (owner, D2) class migration | Move class-method callbacks from fallback A1 to B | Explicit caller-change ledger | class-method rows: caller → anon |
| **A2b** (separate lane) | Argument following to anonymous targets | its own plan | `param.inline-arrow-arg.*` (6) |

S2 is the largest: minting, locals, `this`, bytes. If its forecast goes over one review-able unit, split it into S2a (minting plus R4c locals: both halves of the soundness pair must land together) and S2b (`this` inheritance plus bytes). The prototype diff (`measure/prototype.diff`, 16 lines) shows that minting is small. The R4c locals chain is the review risk.

## 10. Risks

- **False Exacts from missing scope facts.** The R4c gap is shown. Other per-caller facts keyed by caller `FunctionId` may have the same hole: `method_class_span`, receiver vars, caller locals for P6-lite. S2 must list every `*.get(&site.caller)` in `resolution*.rs` and either give anonymous owners the fact or inherit it along the chain.
- **Silent re-seeding of algorithms.** If the S1 guards are incomplete, membrane, spiral and echo change results without a test failing. S1 lands first, and S2's gate includes their test suites at RAYON_NUM_THREADS=1 and 4.
- **Cost:** +31–41% wall time on X is mostly test-code records, and it is unprofiled (D5).
- **Taint false positives** from new Step-5b edges inside callbacks: measured in S2, not predicted here.
- **Wire changes** in S0 (rename) and S3 (`caller.anon`, labels): additive except the rename.
- **Precision checks on the prototype are partial.** 473 X Exact targets could not be checked: the checker does not resolve X's path aliases in this configuration. The disagreements were spot-checked, not adjudicated.

## 11. Owner decisions

- **D1. Naming.** `<anon@L:C>` as the key prefix (rename PR-B in S0), plus `label` as display only? (Recommended: yes.)
- **D2. Class methods.** Keep fallback A1 (the method keeps the calls in its callbacks, as today), or migrate to B later (S5) and accept about 1,000 existing records per corpus changing caller? (Recommended: A1 now, S5 only if a consumer needs it.)
- **D3. Anonymous generators.** Add `generator_function` to the synthetic set: B for both the call graph and the DFG pass, which reopens a PR-B surface. Or leave them unrecorded. (Rare: two matrix rows; not separately counted on X/T.)
- **D4. Module-level and class-field creators.** No creates edge plus `enclosing_scope` text (recommended), or a `<module>` pseudo-node like `framework_entries/mod.rs:35`?
- **D5. Cost budget.** Is +31–41% (X) / +20% (T) `call-stats` wall time acceptable for S2, or must S4 (owner-check indexing) land first?
- **D6. Framework entries / #376 item 10.** Map Express inline handlers to the anonymous node in S3, or leave them.
- **D7. `const v = useMemo(() => …)`.** Today the factory arrow is named `v`, the memoized *value*, by P3. Leave it (recommended for this lane), or treat hook factories as anonymous: label `A › useMemo factory`?
- **D8. Positional key plus structural `anchor`** (recommended), or a structural key?
- **D9. Query surface.** Extend `nav functions` with `--file/--anonymous/--in` (recommended), or add a new `nav anon` query?
