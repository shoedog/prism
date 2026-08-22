# Design — Java/TypeScript parameter materialization (PR A2; prerequisite split out of Item A, 2026-08-21)

## Problem (grounded by the Item A review, gpt-5.6-sol)
Neither parameter-name extractor in `src/ast.rs` (text: ~L6381-6440; byte-bearing: ~L6442-6500) handles Java
`formal_parameter` or TypeScript `required_parameter`/`optional_parameter`, so Java/TS functions have NO parameter
Def endpoints: `compute_param_names` → Step-5b arg→param edges never exist, taint parameter seeding
(`src/reasoning/seeds.rs:253-276`) sees nothing, one-hop parameter-passed callback resolution
(`src/call_graph.rs:1827-1883`) cannot fire. A first attempt (preserved on local branch
`java-ts-param-materialization-wip` in `~/code/slicing-p4-multiline-args`, commit 1cd3322) materialized the simple
identifier forms but (WRONG-1) ignored TS `pattern` so destructured params were dropped and later params SHIFTED LEFT
(false arg→param edges, wrong callback slots) and (WRONG-2) left Java varargs `spread_parameter` / TS `rest_pattern`
without Defs.

## Design
1. **Positional-slot contract (the invariant):** the extractors return one entry per RUNTIME parameter slot, in
   declaration order; an unsupported/unrepresentable slot yields an explicit placeholder (e.g. `Option<String>`/
   `None` or a sentinel the consumers treat as "no binding") — NEVER compression. All consumers
   (`compute_param_names`, Step-5b, `function_parameter_occurrences`, one-hop callback resolution, taint seeding)
   must index by slot and skip placeholder slots (fail closed: no edge for that slot; the other slots stay aligned).
   Audit every consumer (grep `function_parameter_names|function_parameter_occurrences|compute_param_names`).
2. **Java** (tree-sitter-java 0.23.x): `formal_parameter` → the `name` field identifier (not `type_identifier`/
   `modifiers`/`dimensions`); `spread_parameter` (varargs) → its identifier (`variable_declarator`/`identifier` child per
   grammar); `receiver_parameter` (`Foo this`) → NOT a runtime slot (exclude). Annotations/final modifiers ignored.
3. **TypeScript/TSX** (tree-sitter-typescript 0.23.x): `required_parameter`/`optional_parameter` → field `pattern`:
   `identifier` → name; `rest_pattern` wrapping an identifier → name (`...p`); `object_pattern`/`array_pattern`
   (destructuring) → placeholder slot (fail closed; no binding name, byte span kept for the slot if useful);
   `this` parameter → NOT a runtime slot (exclude); default values (`optional_parameter` with `value`) → the binding
   name, NEVER an identifier from the initializer. JS (`formal_parameters` children) — confirm current behavior is
   unchanged and slot-aligned.
4. **Safe failure direction:** fail closed per slot; misalignment is the forbidden failure (it mints false edges).
5. **Cache:** parameter names/occurrences feed the persisted DFG/CPG and the CallGraph (callback resolution) →
   ONE `CACHE_VERSION` bump; reconsider the resolved-call-edge sidecar version per its topology-change convention
   (sol: a separate bump is not strictly required for stale-data safety, but its history bumps on topology changes).
6. **Tests (both poles, per grammar form):** Java simple / varargs / receiver-excluded / annotated / array-typed;
   TS simple / optional / default / rest / destructured-object / destructured-array / `this` / mixed
   (`f({x}: T, y)` → slot 1 == `y`, slot 0 placeholder; an arg→param edge for `y` from argument index 1 ONLY; callback
   slot test: `invoke({x}, cb)` resolves `cb` from argument 1). Cross-check: existing Python/Go/Rust/JS param tests
   byte-identical. Multi-line Java/TS endpoint tests (moved from Item A) once A1 has merged.
7. **Measurement (acceptance):** `tier-a --matrix-only` 0 regr; `prism nav call-stats` on a TS corpus (the
   `TypeScript` bench repo or excalidraw) and a Java corpus (guava) before/after: new one-hop callback Exact edges
   counted under their kind; Python/Go/Rust byte-identical; a `taint_reaches` smoke on a TS fixture with a
   destructured first param proving NO edge to the shifted slot.

## Non-goals
Destructured-binding name extraction (binding `x` from `{x}`), default-value flow, Java lambda parameters
(separate node kinds — out of scope unless already covered), C/C++ (untouched).

---

# Spec review rev-1 (gpt-5.6-sol @ xhigh, read-only, 2026-08-21) — VERDICT: FIX (design must be revised before any implementation)

Controller disposition: PARKED as an architectural item (scope grew: canonical parameter-slot model across ALL languages + Level-3 callback-resolver identity + rest/variadic/spread binding semantics + cache/sidecar + parse-recovery). NOTE the pre-existing precision defects on main that this review exposed — JS default/rest parameter compression can mint a false Level-3 callback edge TODAY (`function invoke(x = 0, cb)` + `invoke(safe, 0)` → false `cb → safe`); Go grouped/variadic params extract only the first identifier; Level-3 finds the containing function by NAME only and uses raw name buckets/line-based argument lookup. These are new follow-up-queue entries (precision).

I’ll perform a strictly read-only review: no edits, builds, tests, network, or provider-backed analysis. I’m using the local `lsp-nav` and `prism-nav` guidance because this review hinges on an exhaustive consumer/blast-radius audit; if their semantic tooling is unavailable or would exceed the cap, I’ll fall back to repository-grounded `rg`/source inspection and say so. Review cap: two passes—mechanism/grammar first, then consumer/test/cache falsification.The memory registry had no prior entry for this branch or change, so I’m treating the checkout as the sole authority. The navigation skills confirm that literal source reads remain necessary: Prism is name-based and the LSP server is not available in this session; I’ll use one bounded local Prism caller query only to sharpen the exhaustive `rg` audit.First-pass falsification has already found two concrete design hazards: the new slot model must be canonical rather than a filtered name list, and spread/variadic calls need their own binding rule. I’m now checking whether the local structural index finds any consumer that literal symbol search missed, then I’ll pin those hazards to grammar and cache mechanics.The structural index matched the literal audit: six production callers of `function_parameter_names`, three of `function_parameter_occurrences`, and two production Step‑5b callers of `compute_param_names`; no extra semantic caller was found. The important result is that several of those callers are positional in different ways, so “skip placeholders” cannot be implemented as one generic filter.Confirmed clean `HEAD` is exactly `47e21aee9f9443efcb41086799ba30e369814c8c`; the WIP branch is its direct child `1cd3322`. The pinned grammars are Java `0.23.5` and TypeScript `0.23.2` (`Cargo.lock:979-982`, `Cargo.lock:1035-1038`).

The core grammar reading is mostly correct:

- Java `formal_parameter` exposes `name`; `spread_parameter` contains a `variable_declarator`; `receiver_parameter` is distinct (`tree-sitter-java-0.23.5/src/node-types.json:2034-2080,2976-3003,3332-3358`).
- TS `required_parameter`/`optional_parameter` use `pattern`, with a separate `value` initializer; `rest_pattern` may contain identifiers or destructuring forms (`tree-sitter-typescript-0.23.2/common/define-grammar.js:659-678`; `typescript/src/node-types.json:3934-3990,4371-4475`).

## Ranked change list

1. **WRONG — “JS unchanged and slot-aligned” is impossible; unchanged JS already mints false callback edges.**

   JavaScript formal parameters include `assignment_pattern`, ordinary patterns, and `rest_pattern` (`tree-sitter-javascript-0.23.1/grammar.js:780,1146-1172`; `src/node-types.json:1451-1466`). HEAD’s extractor has no `assignment_pattern` or `rest_pattern` arm, and both public APIs compress every `None` (`src/ast.rs:5868-5904,6378-6484`).

   Constructible failure:

   ```javascript
   function invoke(x = 0, cb) { cb(); }
   function safe() {}
   invoke(safe, 0);
   ```

   Extraction returns `["cb"]`; Level 3 finds `cb` at index 0 and resolves it from `safe` (`src/call_graph.rs:1865-1885`). Runtime `cb` is `0`, so the synthetic `cb → safe` edge is false.

   The global invariant also conflicts with “Go byte-identical”: Go permits multiple names in one declaration and a separate variadic declaration (`tree-sitter-go-0.23.4/grammar.js:240-258`), while the current extractor returns only the first identifier from each declaration. Either:

   - extend the slot repair to JS and grouped/variadic Go, or
   - explicitly narrow the invariant to Java/TS and stop claiming every runtime slot or global no-misalignment.

   Confidence: high; both are grammar-valid, production-reachable forms.

2. **WRONG — the proposed `Option<String>`/sentinel shape is insufficient to encode the required semantics safely.**

   There are three distinct states:

   - no runtime slot: Java receiver / TS `this`;
   - runtime slot with no representable binding: destructuring;
   - named runtime slot, possibly variadic.

   A single `None` cannot distinguish the first two. For `f(this: T, {x}, y)`, filtering `None` shifts `y` left; retaining every `None` incorrectly counts `this`.

   Require one canonical representation, such as:

   ```rust
   enum ParameterChild {
       NotRuntime,
       Slot(ParameterSlot),
   }

   struct ParameterSlot {
       binding: Option<ParameterOccurrence>,
       cardinality: OneOrZero | Variadic,
   }
   ```

   Do not use a magic string. Current consumers assume real strings and construct paths or identifiers from them (`src/data_flow.rs:265-287`, `src/algorithms/quantum_slice.rs:584-592`, `src/algorithms/peer_consistency_slice.rs:188-207`).

3. **WRONG — materializing rest/varargs names does not define their argument-binding semantics.**

   Step 5b currently binds exactly `argument[i] → parameter[i]` once (`src/cpg/build.rs:880-905`). Therefore:

   ```java
   sink(clean, tainted); // sink(String... xs)
   ```

   binds only `clean → xs`; `tainted` is lost. TS rest parameters have the same problem.

   Conversely, dynamic JS/TS call spreads can mint false edges:

   ```javascript
   function invoke(a, cb) { cb(); }
   function safe() {}
   invoke(...[0, () => {}], safe);
   ```

   The runtime callback is the anonymous function, but ordinal extraction treats `safe` as argument 1. Call extraction only marks Go’s `variadic_argument`, not JS/TS `spread_element` (`src/ast.rs:5622-5638`), and neither Step 5b nor Level 3 gates on spread uncertainty.

   Specify:

   - every statically present trailing argument maps to a Java/TS rest parameter;
   - positions at or after a dynamic spread fail closed;
   - only positions before the first spread remain deterministic;
   - both production and serial-reference Step 5b paths receive the identical rule.

4. **WRONG — Level-3 callback resolution still ignores exact function and call-site identity.**

   It:

   - finds the containing function by name only (`src/call_graph.rs:1835-1838`);
   - reads all inbound calls from a raw name bucket (`src/call_graph.rs:1870-1877`; raw insertion at `src/call_graph.rs:1172-1180`);
   - selects an argument by line and bare callee name rather than `start_byte` (`src/ast.rs:6545-6595`).

   With `A.invoke` and `B.invoke` in one TS file, a call only to `B.invoke(0, safe)` can be used while processing the callback inside `A.invoke`, creating an unreachable `A.cb → safe` edge.

   Require exact `(file, name, start_line)` function lookup, retain only incoming sites whose resolved target is that exact `FunctionId`, and retrieve arguments using the `CallSite.start_byte`. Ambiguous or non-exact inbound resolution must not authorize an Exact indirect edge.

5. **WRONG — the stated Java callback benefit is not supported by the current mechanism.**

   Java invokes a callback parameter as `cb.run()`/`cb.apply()`. Level 3 compares the call’s `callee_name` (`run`/`apply`) with parameter names such as `cb` (`src/call_graph.rs:1844-1867`), so Java parameter materialization alone cannot activate this path.

   Narrow the benefit and corpus expectation to TS, or add a separately designed qualifier-is-parameter callback mechanism. Java materialization remains useful for DFG/taint.

6. **SMELL — the consumer migration must be explicit, because “skip placeholder” means different things at different sites.**

   The complete production census is:

   - positional: `compute_param_names` and both Step-5b loops (`src/cpg/build.rs:20-63,894-905,1008-1019`);
   - positional callback lookup (`src/call_graph.rs:1835-1885`);
   - positional downstream index: Python primitive slicing (`src/algorithms/primitive_slice.rs:317-347,407`);
   - first-slot semantics: peer consistency (`src/algorithms/peer_consistency_slice.rs:188-207`);
   - name sets: quantum slicing (`src/algorithms/quantum_slice.rs:584-592,662-674`);
   - named occurrences only: DFG and reasoning seeds (`src/data_flow.rs:265-287`; `src/reasoning/seeds.rs:253-267`);
   - cached fallback: `FunctionInfo.param_names` (`src/ast.rs:95-103,521-534`).

   Positional consumers must enumerate slots before skipping missing bindings. Name-set/Def/seed consumers should flatten only named bindings. A placeholder first slot must make the peer consumer skip the function, not promote the second slot.

7. **SMELL — fail-closed behavior is incomplete for parse recovery.**

   `ParsedFile::parse` retains errorful trees (`src/ast.rs:274-306`), and repository loading only drops files above a 30% error rate (`src/repo_loader.rs:125-136`). An `ERROR`/missing node may cover an unknown number of parameters; treating it as one placeholder does not prove alignment.

   Positional binding should refuse the whole parameter list—or at least the unknown suffix—when the parameters subtree contains `ERROR`, missing nodes, or an unsupported child with unprovable cardinality.

8. **SMELL — cache handling is safe but should be pinned, not “reconsidered.”**

   Bump CPG cache `40 → 41` (`src/cpg_cache.rs:118-129`). The build-input fingerprint already prevents stale-data service (`src/cpg_cache.rs:142-145,430-438`).

   A separate navigation sidecar bump is not required for stale-data safety: it only loads after a CPG hit and checks the same fingerprint (`src/navigation/call_edge_cache.rs:117-140,169-181`). Nevertheless, every prior topology change bumped its explicit version (`src/navigation/call_edge_cache.rs:10-37`), so this design should require `10 → 11` for convention, diagnostics, and its pinned version test.

9. **SMELL — the proposed tests and measurements do not cover the mechanisms above.**

   Add both positive and negative assertions for:

   - canonical slot vectors and byte spans;
   - JS default/rest/destructuring and grouped/variadic Go if the invariant remains global;
   - every trailing argument into Java varargs/TS rest;
   - fail-closed JS/TS call spread;
   - TS `this` and Java receiver followed by a real parameter mapping from argument 0;
   - TSX;
   - exact-target callback resolution with same-name qualified functions and same-line calls;
   - positive and negative taint reachability for the corrected slot;
   - full-build versus incremental-cache parity and sidecar invalidation.

   The rejected branch’s parameter-specific tests only prove a simple Java/TS Def exists (`java-ts-param-materialization-wip:src/cpg/multiline_call_arg_tests.rs:433-465`).

   `call-stats` also cannot currently identify “parameter callback” as a distinct kind: synthetic sites retain only `origin = IndirectResolution` (`src/call_graph.rs:1922-1943`), while telemetry aggregates resolved kinds such as `free_single` (`src/navigation/queries.rs:214-270`). Add origin-stratified telemetry or record the exact added site set.

   Finally, acceptance omits the required Tier-A quick run and immediate release rebuild. The repository mandates all three commands (`AGENTS.md:1-14`), plus the global full-suite requirement. No tests or measurements were run in this read-only review.

**VERDICT: FIX**
