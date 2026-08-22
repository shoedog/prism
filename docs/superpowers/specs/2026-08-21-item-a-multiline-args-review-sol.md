I’ll do a two-pass read-only review: first every diff hunk in context, then one focused falsification pass over call-site matching, parameter extraction consumers, cache effects, and test discrimination. I’m using the `lsp-nav` guidance only to structure exact-reference checks; I will not edit, build, test, invoke network/providers, or run anything that may write.The prior-memory registry has no entry for this branch or change, so I’m grounding the review entirely in the current checkout and supplied controller evidence. The repository doctrine reinforces the key review invariant here: ambiguous framework/data-flow bindings must fail closed because false edges are worse than missed edges.The first pass confirms the selector is shared by production and serial collectors, and the span is correctly treated as half-open (`end - 1` for the last line). One important precision point is already clear: same-line behavior is not literally byte-identical—containment can suppress a legacy edge when the one-node-per-key index retained a different same-line occurrence—but that change is fail-closed, not a false binding. I’m now checking whether any new path can fail open.The parameter materialization is broader than “DFG only.” Both extractors feed the eager function table and `function_parameter_names`; the call-graph builder uses that API for one-hop parameter-passed function-pointer resolution. A Java/TypeScript callback parameter can therefore create new persisted CallGraph and navigation edges. I’m checking the pinned grammars now to distinguish a correct-but-separate expansion from any constructible misbinding.The falsification pass found a concrete blocker in addition (ii): TypeScript destructured parameters are omitted from the returned vector, so later parameters shift left. For `f({x}: T, y)` the code maps argument 0 to `y`; the same compression can also make the indirect-call resolver use the wrong callback argument. The trace containment gate itself survives the nested-call analysis: it filters by the target callee-name bucket, requires exactly one site, and fails closed for same-name nesting.## Findings

1. **WRONG — TypeScript destructuring collapses parameter positions and creates false arg→param/call edges.** The pinned TypeScript 0.23.2 grammar stores ordinary parameter bindings in `pattern`, which may be an identifier, object/array pattern, rest pattern, or `this` (`Cargo.lock:1035`; installed `common/define-grammar.js:659-678`). The new extractors ignore `pattern`; after checking `name`, they accept only an immediate `identifier` (`src/ast.rs:6422-6433`, `src/ast.rs:6475-6486`). For:
   ```ts
   function f({x}: T, y: string) { sink(y); }
   f(attacker, safe);
   ```
   the first parameter returns `None`, the vector compresses to `["y"]`, and Step-5b maps `attacker`—argument 0—to `f.y` (`src/cpg/build.rs:903-947`). That is a false taint edge. The same compression corrupts parameter-passed callback resolution: `invoke({x}, cb)` treats argument 0 as `cb` at `src/call_graph.rs:1835-1883`, potentially resolving the callback to the wrong function. Defaults are also hazardous because the fallback can select a direct identifier from the initializer rather than a binding. High confidence. Preserve positional slots, distinguish the non-runtime `this` parameter, and fail closed when a runtime slot cannot be represented.

2. **WRONG — Java varargs and TypeScript rest parameters still have no Def endpoint.** Java 0.23.5 emits `spread_parameter`, distinct from `formal_parameter` (`Cargo.lock:979`; installed Java `grammar.js:1215-1245`), but only `formal_parameter` was added at `src/ast.rs:6381` and `src/ast.rs:6442`. TypeScript `...p` exposes a `rest_pattern`; the immediate-identifier fallback does not unwrap it. Consequently:
   ```java
   static void f(Object... p) { sink(p); }
   f(
       user
   );
   ```
   and the equivalent TypeScript `function f(...p)` materialize no parameter Def at `src/data_flow.rs:265-293`, emit no Step-5b edge, and report `NotReached` through `sink(p)`. High confidence. Add explicit identifier-only extraction for Java `spread_parameter` and TypeScript `rest_pattern`; keep destructured rest patterns fail-closed.

3. **SMELL — addition (ii) should be split into its own prerequisite PR.** It changes much more than multiline Step-5b:

   - public `function_parameter_names`/`function_parameter_occurrences` behavior;
   - eager function metadata (`src/ast.rs:520-534`);
   - DFG and CPG parameter Defs (`src/data_flow.rs:258-293`);
   - symbol/parameter taint seeding (`src/reasoning/seeds.rs:253-276`);
   - one-hop indirect CallGraph resolution (`src/call_graph.rs:1827-1883`);
   - therefore persisted CallGraph and resolved navigation edges.

   Simple Java and TypeScript identifier parameters are extracted correctly, but this blast radius needs grammar-form tests and independent DFG, seed, CallGraph, and nav measurements. Land it first, then base the multiline PR on it.

4. **SMELL — the tests omit the exact grammar and trace risks introduced here.** The Java/TypeScript materialization tests cover only simple `p` (`src/cpg/multiline_call_arg_tests.rs:432-465`); there are no destructuring, default, rest/varargs, Java receiver, or TypeScript `this` cases. The wrapper equality test (`src/cpg/multiline_call_arg_tests.rs:509-520`) is likely vacuous for Python/Go parentheses, Java casts, and JS/TS spread because `AccessPath::from_expr` does not normalize those forms; both edge sets can be empty. Only Rust’s leading `&` is normalized by `arg_text`. There is also no taint-descent test for distinct-name or same-name nested multiline calls; the nested test checks graph edges only (`src/cpg/multiline_call_arg_tests.rs:572-585`).

## Requested assessments

- `argument_var_node_in_span` is sound and fail-closed. It correctly derives the inclusive line range from the half-open span (`src/cpg/build.rs:971-975`), applies per-line Use→Def ordering, byte containment, and exactly-one selection (`src/cpg/build.rs:976-1016`). Both production and serial paths call this same selector (`src/cpg/build.rs:922-933`, `src/cpg/build.rs:1084-1095`).

- Same-line behavior is not literally byte-identical. R-value spans are byte-sorted (`src/ast.rs:3986-3988`), and `var_index` keeps the first occurrence per key (`src/cpg/build.rs:491-502`). For two same-path occurrences on one line, containment can now reject a later argument because the retained first occurrence lies outside its span. That drops a legacy edge but cannot bind the wrong occurrence, so the change is acceptable fail-closed.

- Nested `g(h(\n user))` does not mint `user→g.param`: `AccessPath::from_expr` sees the outer argument as the call expression, not `user`; only the inner argument derives `user`. The exact inner-present/outer-absent test pins this.

- The trace gate is sound. It first selects `callers[target_fn]` (`src/cpg/trace.rs:608-612`), so distinct `g` and `h` spans do not compete for an edge targeting `h`. It requires exactly one matching site (`src/cpg/trace.rs:613-631`); same-name nesting matches both containing spans and refuses descent. For a particular same-line site, the byte arm adds nothing because the line arm already matches. It can additionally match an enclosing same-name site beginning on another line, but that produces ambiguity and refusal, not a false descent.

- `source_start_byte` only changes the in-memory per-trace `BTreeMap` key (`src/cpg/trace.rs:12-21`, `src/cpg/trace.rs:594-605`). It is not persisted.

- Cache 40→41 once is correct and sufficient for the serialized CPG, DFG, and CallGraph (`src/cpg_cache.rs:129-131`, `src/cpg_cache.rs:292-307`). However, the v41 description is incomplete because addition (ii) can change CallGraph/nav topology, not only DataFlow. The resolved-call-edge sidecar is protected by binary-input fingerprinting and only loads after a CPG cache hit, so a separate bump is not required for stale-data safety; its version should nevertheless be reconsidered if (ii) remains, consistent with the sidecar’s topology-change history.

- `call_argument_texts_at` remains text-identical and retains its public signature (`src/ast.rs:6616-6643`). The reasoning consumer remains text-only and unchanged (`src/reasoning/seeds.rs:358-373`).

- No Rust public signature was broken; the new span helper is `pub(crate)`. The existing public parameter APIs do change observable Java/TS/TSX behavior.

- New test files respect the cap: 587 and 29 lines. Existing changed production/test modules remain legacy over-cap.

- Independently observed: branch/base/commit match the prompt; `git diff --check` passed. Supplied evidence says release build plus Tier-A matrix 104/104, zero regressions. Full `cargo test` totals, `cargo fmt --check`, call-stats comparisons, and the taint smoke were not supplied or independently run under the read-only restriction. The pristine-main red pole is therefore code-grounded, not independently executed.

## Ranked must-change list

1. Preserve TypeScript runtime parameter ordinals; never compress an unsupported destructured slot or select an initializer identifier.
2. Handle Java varargs and identifier-bound TypeScript rest parameters, with exact positive/negative endpoint tests.
3. Split Java/TS parameter materialization into a prerequisite PR and measure its DFG, seeding, CallGraph, nav, and cache effects.
4. Add non-vacuous wrapper assertions plus nested multiline descent/refusal tests, then provide the missing full-suite, fmt, call-stats, and smoke evidence.

VERDICT: FIX
