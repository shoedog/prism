**Verdict: FIX — round 1 of at most 2.** Fold W1–W3 and S1 into this preserved prototype/spec, confirm their regressions and the revised measurement gates, then proceed. These are bounded repairs; a restart or general redesign is unnecessary. The current rule is not ready to land unchanged.

**Findings list**

1. **W1 — WRONG / MATERIAL:** Arm (b) drops a genuine script-global declaration called from a module.
2. **W2 — WRONG / MATERIAL:** TypeScript `import f = require(...)` is missing from the exemption evidence, losing a static import/export binding.
3. **W3 — WRONG / MATERIAL:** Indirect member-call records test import evidence in the wrong file and lose a correctly constructed callable.
4. **S1 — SMELL / MATERIAL:** The arm (a) alias-preservation control does not assert a resolution; its binding-reason boundary is not pinned by the new tests or four mutants.

**Scope and custody.** Reviewed `/Users/wesleyjinks/code/prism-e5-plan`, clean branch `plan/js-e5-call-referenceability`, HEAD `ce6de39722c0211d68de3be2d35361e14408731f`, against main `bab0dfbdcf97ae6dcccc8d8df66c01c9dddf16e7`. Read the five required packet files, the complete specified src/tests/mutants diff, the Exact contract, relevant import extraction, constructors, cache loading, and incremental/MCP seams. No other review report was read. No repository edits, Git writes, corpus package execution, network, agents, or directory named `frontend-portal` were used.

Frozen `base-prism` SHA-256 is `6e3c9e259967f7b6cbc94049dd5d55cac4ab326ae16e39a6ff20e474cb3d1bf2`; `head-prism` is `38b923a338595da3e2a2c482fd0fbd733b56e08d8ed655a3ae09c2b66e571f87`. Both match their binding manifests. All frozen non-test src hashes match the reviewed checkout; Cargo files and build.rs also match. The supplied base/main equality receipt was inspected. All executable comparisons below use those two binaries in the same environment on reviewer-written fixtures, with a new cache directory for every invocation.

**W1 — WRONG / MATERIAL: module syntax does not exclude an outer global declaration.** Confidence: 99/100.

Concrete fixture, run in `sol-scratch/fixtures/module_global_js`:

```js
// global.js, a Script in the same program
function f(x) { return x + 1; }
// caller.js, a Module
export function caller(x) { return f(x); }
```

Base resolves `caller → global.js:f`, `free_single`; head returns only an unresolved `f` placeholder. The equivalent `.ts` fixture has the same loss. This is a declaration/scope binding: a module can resolve an unshadowed identifier through its outer global environment. It is not an injected host function. The small Node VM fixture creates an empty context, evaluates the in-repo Script, checks `f.toString()` against that declaration, and observes `caller(7) = 8`. Script-caller and hoisted same-file controls retain their targets in both binaries, ruling out a general parser or symbol-discovery loss.

Evidence: `src/resolution.rs:3675–3681` does not inspect target declaration scope; `src/call_graph_js_referenceability.rs:19–26` treats the caller's module syntax plus an empty import set as sufficient negative evidence. `SPEC-e5.md:15` specifies that same rule. `CLAUDE.md:233–239` defines the static-binding contract; the host-global exclusion does not turn a declaration in another Script into a host injection. Exact base/head commands and JSON outputs are in `sol-scratch/probe-results.json`, cases `module_global_js` and `module_global_ts`; synthetic identity evidence is in `synthetic-bindings.cjs` and `synthetic-bindings.stdout`.

Recommended fix: preserve an unshadowed call to a syntactically global, top-level Script declaration. Retain target-side global-declaration facts and require the caller's own binding to be unbound before applying this exception; a parameter/local alias must still be refused. An absent entry in the negative-target set is not itself proof of global scope. Add module→Script positive controls and shadowed/nested/module-target negative twins. Update the existing module-refusal table, which currently expects this valid global case to disappear (`src/resolution_e5_bound_tests.rs:43–52`).

Options: the precise declaration-scope exception keeps more false-positive yield. Conservatively exempting possible Script targets at the unbound boundary is simpler but retains more uncertain fallback edges. Explicitly excluding module→Script globals from the contract would avoid changing the predicate, but would be a disclosed recall/contract change and cannot be called zero correct-binding loss.

When this would not apply: a program model explicitly forbids shared Script globals, or the target is actually a CommonJS/module-local/nested declaration. This finding assumes a true Script global in the same program, not every file lacking import/export tokens.

**W2 — WRONG / MATERIAL: the import-equals binding is invisible to the new evidence mask.** Confidence: 100/100.

Concrete fixture, run in `sol-scratch/fixtures/import_equals_only`:

```ts
// callee.ts
function f(x: number) { return x; } export = f;
// caller.ts
import f = require('./callee');
function caller(x: number) { return f(x); }
```

Base resolves `callee.ts:f` through Exact `free_single`; head drops it. Adding `export` to `caller` has the same result. The import-equals declaration alone arms the module guard. Here the exported value and local imported name have a direct static identity. The new rule mistakes an unhandled import syntax node for absent import evidence.

Evidence: `src/ast.rs:1703–1723` requires a `source` field on the outer import statement and handles import clauses/identifiers, but not `import_require_clause`. The vendored TypeScript `node-types.json:2835–2857,2891–2921` puts the required `source` and local identifier inside that clause. E5 then uses only `extract_imports().into_keys()` (`src/call_graph_js_referenceability.rs:26`). See `probe-results.json`, `import_equals`, and `probe-extra-results.json`, `import_equals_only`. A recognized nested scalar-require peer retains Exact `free_single` in both binaries, separating this missing syntax form from blanket require refusal.

Recommended fix: include import-require-clause local names in E5's conservative evidence extraction. Prefer an E5-specific name collector if changing the shared import-path extractor would affect unrelated consumers. This exemption need not prove or implement positive `export =` resolution. Test the exact fixture above, the version with another root export, a renamed local, and an unrelated imported name that must not exempt the call. Assert target, kind, confidence, and the refusal reason on negative cases.

Options: extending the shared extractor benefits other consumers but requires checking its positive-resolution effects. An E5-only collector has a smaller behavioral scope. Suppressing arm (b) for the entire file when this import form appears is a safe interim fallback with lower precision. Full import-equals resolution is a separate, larger enhancement.

When this would not apply: TypeScript import-equals is explicitly unsupported with intentional recall loss, or an earlier positive rung already resolves the site. Neither condition holds for this fixture or the present preservation claim.

**W3 — WRONG / MATERIAL: indirect provenance is replaced by caller-local spelling.** Confidence: 99/100.

Concrete fixture, run in `sol-scratch/fixtures/ordinary_indirect`:

```js
// item.js
export function f(x) { return x; }
// box.js
import { f } from './item';
export function Box() { return null; }
Box.Item = f;
// caller.js
import { Box } from './box';
export function caller(x) { return Box.Item(x); }
```

Base's indirect record resolves to `item.js:f`, Exact `free_single`; head removes that target. `caller.js` correctly imports `Box`; it need not import the name `f`, which was written and imported in `box.js`. The synthetic Node fixture checks `Box.Item === item.f` and `caller(7) = 7`. A direct `Box()` control resolves correctly in both binaries, ruling out root-import failure. The JSX twin uses `Box.Item = Component` and `<Box.Item />`; base resolves `Component`, head loses it. The first JSX fixture used the same property/target name and exercised an existing indirect-generation shortcut instead; it produced no E5 loss and is not counterexample evidence.

Evidence: `src/call_graph.rs:2856–2910` gathers field assignments across files; `3087–3111` constructs an indirect site with the consuming caller, substituted target name, no qualifier, and `jsx_element=false`. `src/resolution.rs:3675–3681` then consults that caller's evidence for the substituted name. This follows `SPEC-e5.md:15` literally, so it is a specification defect as well as a prototype defect. See `probe-results.json`, `ordinary_indirect`, and `probe-extra-results.json`, `jsx_indirect_renamed`. The public analogue is independently disclosed in `MEASUREMENTS.md:215–221`.

Recommended fix: ship arm (b) on **Source-origin, physically ordinary unqualified calls** in this slice. Preserve indirect and physical member-expression fallback records pending a provenance-aware follow-up. Carry or derive the physical invocation distinction; `origin == Source && !jsx_element && qualifier.is_none()` alone leaves the packet's 23 Source records whose physical member qualifier was lost. Add ordinary-indirect and JSX-indirect preservation tests, and a physical-qualified Source fallback control. Reprice the narrower union while retaining arm (a).

Options and costs: keeping the current rule retains all measured yield but accepts the demonstrated correct loss; disclosure does not repair the premise. The measured arm-(b) Source/ordinary subset retains X 14 W/11 U/1 I, T 201 W/2 I, SB 47 W/123 I, and all 113 private removals. Relative to all of arm (b), it gives up 14 classified WRONG removals and avoids touching 23 UNDECIDED plus 128 INADMISSIBLE removals; these are arm-(b) differences, not a fresh measurement of the final union. Testing the file where the name was written is more precise, but requires retaining assignment provenance, resolving that name there, and binding the property to the consuming receiver/import. Merely changing the filename does not repair the existing property-name-only index; that belongs in a follow-up.

When this would not apply: indirect targets are explicitly outside the preservation contract and correct losses are authorized, or each record already carries a proven target identity that bypasses FreeSingle. This review's indirect records have neither protection. No later reflective mutation is needed to produce the failure.

**S1 — SMELL / MATERIAL: the advertised alias control does not test the refusal boundary.** Confidence: 98/100; no product regression is claimed for this test gap.

Evidence: `src/resolution_e5_tests.rs:119–139` checks only that a parameter site's binding is not `Unproven("unbound")`; it never asserts a resolution. The script-alias control in `src/resolution_e5_bound_tests.rs:129–140` has a Script target outside arm (a)'s negative set. Consequently, deleting arm (a)'s `Unproven("unbound")` conjunct (`src/resolution.rs:3673`) is not distinguished by these controls. By source inspection it also survives the four registered E5 mutants' selector tests. This survival assessment was not executed as a mutant.

A real control was run in `sol-scratch/fixtures/arm-a-alias-control`: Script caller `function outer(o) { const f = o.cb; function caller(x) { return f(x); } }` with module target `export function f(x) { return x; }`. Both binaries retain Exact `free_single` (`arm-a-alias-control.json`). It holds target negative syntax constant while changing the binding reason, and would fail under the broader arm (a). Its retained edge is a deliberate legacy control, not a new assertion of runtime alias identity.

Recommended fix: add that full-result control and a mutation removing the binding-reason conjunct. Add the preservation/boundary regressions required by W1–W3. Options: run this new mutant through the existing scoped gate for durable evidence, or maintain equivalent explicit boundary tests with a documented smaller mutation sample. The first costs another targeted build but directly establishes discrimination.

When this would not apply: the test is relabeled as metadata-only and a separate executable alias boundary control exists. Four successful sampled mutants alone do not establish that control.

**Answers to the six questions.**

1. **Static binding loss:** yes, W1 and W2. Default and renamed imports resolve as `import_member`; namespace calls as `import_qualified`; named imports are covered by the supplied tests. Type-only calls remain unresolved in both binaries. Scalar, destructured, renamed, and nested requires are collected, and nested scalar/destructured controls retain fallback results. Dynamic `await import()` destructuring is not collected and loses its fallback in the probe; runtime re-acquisition is explicitly outside `CLAUDE.md:235–236`, so this is not counted as another WRONG. Re-export-only syntax supplies no local binding: dropping its bare call is appropriate. `.cjs` and `.mjs` without root import/export syntax are unchanged by this deliberately syntactic predicate. `export {}` and TypeScript `export =` arm it. The ambient `.d.ts` fixture has no resolved target in either build, so it demonstrates no new loss. Same-file and hoisted declarations retain `local_def`; earlier positive import rungs retain their behavior.

2. **Indirect/JSX and qualified fallback:** ship the Source/physically ordinary subset, for the reasons and costs in W3. Keep assignment-origin/receiver binding repair in a follow-up. Unqualified Source JSX is also excluded by this choice; this is the explicit scope tradeoff. Physical qualified fallback must use the same preservation boundary as indirect member calls.

3. **Tests:** the supplied, hash-bound main-RED log selects ten groups: seven refusal/lifecycle/consumer failures and three preservation passes. The new shape tables exercise both constructors and multiple languages; actual nested-require FreeSingle controls genuinely test arm (b)'s exemption. Some import/same-file controls instead protect earlier ladder rungs, which is useful but does not independently test the new mask. W1 is currently encoded as an incorrect refusal expectation; W2/W3 lack their necessary preservation fixtures; S1 is the alias gap. The three repaired script fixtures retain their stated scope/reason controls: removing exports leaves the old duplicate-scope, unrelated-parameter, and `with` conditions reachable, with complete result assertions. They do not prove module-global preservation. The six listed arm-(b) b5 changes follow the chosen unsupported-alias policy. For clarity, relative to **main**, ten unimported call values per grammar change; four were already changes under arm (a). All seven b5 scenario fixtures were compared on both grammars (`b5-matrix.json`): every unimported call becomes `LocalBindingUnproven`, and imported L5/L7 retain complete Exact `import_member` targets. Four mutants are useful sampling, not enough to pin the complete predicate: the binding-reason removal above survives; changing arm (b) to Source-only also survives the new E5 tests by reading because their module refusal cases contain no indirect records.

4. **Lifecycle:** no concrete stale/absent-fact path found. Full, skeleton, and direct-subset construction populate both facts; remove/merge drop and replace entries by the appropriate file; bincode persists them. Cache v115 and binary identity reject older artifacts (`src/cpg_cache.rs:523–584`). Full hits reconstruct the retained CallGraph; navigation partial hits rebuild (`src/navigation/cache.rs:60–81`). Incremental CPG rebuild removes/replaces changed facts before scope/indirect recomputation and graph assembly (`src/cpg/build.rs:679–709,762–769`); MCP uses that path through `NavigationIndex::build_incremental_from_previous` (`src/navigation/mod.rs:148–167`, `src/mcp/session.rs:400–425`). The retained navigation cache version 62 is protected by CPG-hit and binary-identity gating. These conclusions are source inspection plus supplied serialization/replacement tests, not a reviewer-run warm-cache or live MCP test.

5. **Measurements:** public target totals reconcile exactly from all 890 adjudications: X 27 W/34 U/37 I, T 201 W/2 I, SB 171 W/418 I; none classified CORRECT. Product module-fact records reconcile the exact arm and physical-invocation tables (`measurement-check.json`). Two React Native rows explain the historical checker/product attribution difference; those historical predicates must not replace the product facts when repricing. U/I are explicitly exposure, not yield. SecBench's 418/589 removals are inadmissible: 71.0% of its removed-target population remains unknown oracle cost, against 171 known WRONG removals. The packet discloses this adequately; it does not establish zero true cost. The graph ledger also discloses 7 CORRECT ReturnContains and 11 CORRECT ReturnInput removals; these are not CORRECT Exact-target losses and are not silently counted as WRONG yield. Private 109 W/4 U and zero additions are supplied aggregate evidence. W1–W3 show why the observed zero classified-CORRECT count cannot certify a universal preservation claim.

6. **Readiness:** resolve the global exception, import-equals evidence, and Source/physical invocation boundary; add the discriminating tests/mutations; rerun the applicable acceptance measurements for the resulting rule. Preserve drop behavior, the union structure, R-local follow-up, and unchanged D12. No other implementation decision needs a general redesign.

**What checked sound.** The two FreeSingle guards share one predicate; earlier proven rungs are not broadly rewritten. Facts are negative/exemption evidence rather than positive target authority. Non-JS isolation and incremental/cache custody are structurally consistent. Public ledgers distinguish target yield, unknown exposure, consumer removals, and navigation placeholders. The scope/reason fixtures remain meaningful after becoming scripts, and the b5 imported controls keep their complete values.

**Execution and limits.** Ran 78 admitted frozen-CLI synthetic invocations, inspecting their JSON outputs, plus the two small Node synthetic identity checks. All CLI invocations had fresh caches under the assigned scratch. The initial 38 CLI attempts used the cache flag at the wrong command level and produced no output: they are explicitly INADMISSIBLE, preserved separately, and excluded from evidence. Expectations, falsifiers, alternatives, and outcomes are retained in `probe-log.md` and the probe artifacts. No product fixes or actual mutant runs were performed.

No cargo tests or full suite were run by this reviewer. Supplied full-suite evidence is **5298 run: 5297 passed, 1 failed, 1 skipped**, not an all-green suite. The failed `parameter_slot_characterization::tests::d7_predicate_and_singular_parameter_rows_are_complete` is acceptance-excluded and has a supplied same-environment main control; I did not rerun that attribution control. Supplied doctests are 2/2, scoped mutants 4/4, adapted matrix 178/178, and fresh TS/JS quick gates passed. I did not independently execute those receipts, the private corpus, corpus packages, warm/partial cache transitions, live MCP refresh, Rust quick, full human-triggered Tier-A, or the four excluded SecBench roots. No supplied historical result was promoted to fresh reviewer execution.

**Final verdict: FIX. Findings: W1 WRONG/MATERIAL; W2 WRONG/MATERIAL; W3 WRONG/MATERIAL; S1 SMELL/MATERIAL.**
