**Verdict: FIX — confirmation round 2 of 2; the cap is reached.** The original failure fixtures are repaired, but the requested boundary checks expose three concrete correct-binding losses. These have bounded conservative fixes on the preserved artifact. No restart or further independent review round is needed: fold the chosen exceptions, add discriminating regressions/mutants, and verify the resulting measurements before the implementation PR proceeds.

**Findings list**

1. **R2-W1 — WRONG / MATERIAL:** Arm (b) drops Source JSX whose local component is a static alias of an imported namespace export.
2. **R2-W2 — WRONG / MATERIAL:** Arm (b) drops a Source `super(...)` call bound by `extends` to a statically constructed callable property.
3. **R2-W3 — WRONG / MATERIAL:** The different-terminal namespace-import limit drops a correct TypeScript import alias through an explicit export alias.

No additional SMELL findings. These findings answer the expressly requested boundary questions; they do not reopen the previously accepted lifecycle, consumer, or ordinary refusal design.

**Custody and scope.** Reviewed clean `/Users/wesleyjinks/code/prism-e5-plan`, branch `plan/js-e5-call-referenceability`, HEAD `c08ea98befd173d28eb583dd815867185aff299d`; fold relative to `ce6de397`, whole product change relative to `bab0dfbd`. Current source is `2783899d`; subsequent commits are packet only. Read both round-1 reports, both controller briefs, the required packet, source/tests/mutant changes, and relevant public receipts. All 315 source entries in `step2b/binary-binding.json` match the checkout. Frozen CLI hashes checked:

| Binary | SHA-256 |
|---|---|
| Base | `6e3c9e259967f7b6cbc94049dd5d55cac4ab326ae16e39a6ff20e474cb3d1bf2` |
| Round-1 head | `38b923a338595da3e2a2c482fd0fbd733b56e08d8ed655a3ae09c2b66e571f87` |
| New head | `a2b4841d51bac8af419766f28a658d94aa0a683e531c36b162dbab0563231e61` |

No repository edits, Git writes, providers/network, agents, corpus package execution, private evidence directories, or directories named `frontend-portal` were used. Writes are confined to the assigned scratch and this report. Comparisons below execute main and head in the same environment, with a fresh scratch cache for every invocation.

**R2-W1 — WRONG / MATERIAL: a static namespace alias is a referenceable Source JSX component. Confidence 100/100.**

Exact fixture: `sol-scratch/fixtures/boundary/jsx_namespace_alias/`. Each file's code is on line 1.

`callee.tsx`:

```tsx
export function F() { return null; }
```

`caller.tsx`:

```tsx
import * as N from './callee'; export function outer() { const F = N.F; function caller() { return <F />; } return caller; }
```

Main returns `callee.tsx:F@1-1`, Exact `free_single`, for `caller`. Both round-1 and new heads return only the unresolved `F` placeholder. Fresh `call-stats` confirms main's `kind_exact.free_single = 1`; new head has no Exact kind and `dropped_local_binding_unproven = 1`.

This is a binding by import, export, lexical declaration, and scope syntax: the namespace import selects this module, its `F` export is this function, and the unwritten local `const F = N.F` supplies that exact tag. No runtime re-acquisition, reflective write, or unknown object is involved. The destructuring twin `const { F } = N` produces the same loss. With the actual sources transpiled by the pinned TypeScript compiler, a synthetic JSX factory receives the very same function object as the callee's exported `F` in both fixtures (`identity-results.json`). That corroborates the syntactic identity; runtime success alone is not the binding argument.

Mechanism: these are physical Source JSX sites, not indirect records. `src/resolution.rs:3669-3678` applies arm (b), while the collected import names contain `N`, not the local alias `F`. The R2 exception cannot save a locally bound alias; arm (a) already excludes JSX. A direct `<N.F />` twin retains Exact `import_qualified` on all three binaries, separating this loss from parser/symbol/import discovery failure. Evidence: `boundary.json`, `boundary-stats.json`, `identity-results.json`, and the emitted synthetic code under `sol-scratch/transpiled/`.

Recommended bounded fix: restrict arm (b) to non-JSX sites for this slice, preserving main's fallback for these tags. Add full FunctionId/kind/confidence/no-drop assertions for both static namespace-alias fixtures in `.jsx` and `.tsx`, with a mutant removing the new JSX exception. Retain the existing imported and indirect controls, and reconcile the source-JSX refusal tests and packet with the chosen policy.

Options and cost: the conservative JSX exception gives up the current public ledger's **5 X WRONG removals** and preserves **5 SB INADMISSIBLE removals**; it needs a fresh capture to confirm the resulting totals. An exemption specifically for scope-bound aliases of imported namespaces could retain the five unrelated local-alias WRONG removals, but needs alias/provenance evidence and corresponding negative twins. Keeping the existing predicate retains measured yield and the demonstrated correct loss. Simply preferring a narrower cut would be a SMELL; this fixture makes it WRONG.

When this does not apply: the tag is bound to an arbitrary `o.component`, a parameter, or another value with no demonstrated identity to the fallback target; the import is type-only; or an earlier positive rung already resolves the tag. The five corpus WRONG local aliases do not establish that every Source JSX alias is wrong.

**R2-W2 — WRONG / MATERIAL: a reserved keyword can bind a real constructor through `extends`. Confidence 100/100.**

Exact fixture: `sol-scratch/fixtures/boundary/super_imported_property/`, again one-line files.

`callee.js`:

```js
export const Base = { super: function(x) { this.value = x; } };
```

`caller.js`:

```js
import { Base } from './callee'; export class Child extends Base.super { constructor(x) { super(x); } }
```

Main resolves `constructor`'s call to `callee.js:super@1-1`, Exact `free_single`. Round-1 and new heads drop it. Fresh telemetry is main `free_single: 1` versus new `LocalBindingUnproven: 1`, with no retained Exact target.

The property contains an ordinary constructible function, the imported `Base` object is the syntactically declared object, and `extends Base.super` establishes the parent constructor. `super(x)` invokes that constructor. The synthetic check verifies `Object.getPrototypeOf(Child) === Base.super` and `new Child(7).value === 7`. No illegal declaration named `super`, arrow-function superclass, mutation, or accidental spelling collision is needed.

Mechanism: `super` is a non-identifier callee, so `js_local_binding_at` returns `Unchecked` (`src/call_graph.rs:5728-5758`). Arm (a)'s unbound conjunct therefore does not apply. Arm (b) does apply to the Source module site; the caller imports `Base`, not the spelling `super`. Treating that absence as negative evidence disregards the actual `extends` binding. Evidence: the same `boundary.json`, `boundary-stats.json`, `identity-results.json`, and emitted fixture files. The explicit non-identifier path separates this from a lost unbound-script exception.

Recommended bounded fix: exempt Source `super` calls from arm (b) pending a real superclass-resolution increment. A `callee_name != "super"` conjunct at this guard preserves the current fallback for this fixture; arm (a) already cannot fire for its `Unchecked` site. Add this cross-file constructor preservation regression, full-result assertions, and a mutant deleting the exception.

Options and cost: the conservative exception restores the **23 SB INADMISSIBLE** Source-super removals in the current ledger, with no classified WRONG yield given up by that ledger. The broader follow-up can resolve constructors from `extends` and refuse unrelated same-spelled candidates; that requires more machinery. Simply barring `super` from all free-function rungs without supplying its actual parent binding would still lose this correct target.

When this does not apply: the superclass is an unrelated constructor, the same-spelled function is not its value, or the property is nonconstructible. Those cases do not justify the target. The 23 corpus rows being INADMISSIBLE does not prove this entire invocation form has no correct binding.

**R2-W3 — WRONG / MATERIAL: unequal terminal spelling does not disprove an import alias's identity. Confidence 100/100.**

Exact fixture: `sol-scratch/fixtures/boundary/namespace_renamed_static/`, one-line files.

`callee.ts`:

```ts
export namespace N { export function f(x: number) { return x; } export const renamed = f; }
```

`caller.ts`:

```ts
import { N } from './callee'; import f = N.renamed; export function caller(x: number) { return f(x); }
```

Main retains `callee.ts:f@1-1`, Exact `free_single`. New head drops it with `LocalBindingUnproven`. TypeScript 5.9.3 checks these exact two files with **zero diagnostics** (`namespace-typecheck.json`). The explicit export initializer establishes `N.renamed === N.f`; the import-equals declaration binds local `f` to that value. A synthetic check of compiler-emitted code confirms both object identities and `caller(7) = 7` (`namespace-identity-result.json`). This is static import/export syntax, unlike the separately excluded dynamic-import re-acquisition cases.

Mechanism: `src/call_graph_js_referenceability.rs:44-54` correctly reads the terminal token, but requires `local == terminal`. Here `f != renamed`, so E5 omits the import name. The binder reports an import, so arm (a) does not fire and R2's unbound exception does not apply. Arm (b) then discards a correct resolution. The spec explicitly says other renamed namespace aliases remain refused; that stated limit has a concrete correct cost.

The same-terminal `import f = N.f` control retains the target on both binaries. The negative twin `import f = N.g`, where `g` is a separately declared function, drops main's incorrect fallback to `f` on the new head. These distinguish a valid alias chain from a genuinely different callable. Complete commands, outputs, and telemetry: `namespace-limits.json`.

Recommended bounded fix: count well-formed namespace import-alias locals as E5 exemption evidence regardless of terminal spelling, retaining main rather than minting a new positive target. Keep the exported-require recovery checks separate. Add this full-result regression and revise E5-10/E5-15 and their negative expectations so the mutation gate pins the chosen exemption policy.

Options and cost: that conservative exemption also retains the **two currently WRONG T `format` rows** that R1 intentionally refused; other unpriced exposures require re-measurement. A target-bound export-alias proof can preserve this input while continuing to refuse `N.g`/the `format` rows, but introduces cross-file alias/path evidence beyond the present collector. Keeping terminal equality retains that yield and this demonstrated correct loss. The smaller conservative exemption is sufficient to fix the bounded review item; a general alias resolver is unnecessary for this slice.

When this does not apply: `N.renamed` actually denotes another function or a non-callable value, as in the `N.g` twin. A main fallback to the same local spelling is not inherently correct; the explicit export-to-function identity is essential to this finding.

**Question 1 — round-1 closure table.** “Closed” below concerns each original demonstrated failure or test/packet defect under the controller rulings. It does not claim the resulting boundary is universally safe; R2-W1–W3 above remain material.

| Original finding | Closure | Evidence |
|---|---|---|
| sol W1 | Closed | Reviewer reruns `module_global_js`, `module_global_ts`, empty/export-equals peers: main target restored. R2 exception and `e5_unbound_module_to_script_global_stays_exact_with_negative_twins`; supplied E5-12/16/17/18 killing assertion receipts. |
| sol W2 | Closed | `import_equals` and `import_equals_only` rerun: main target restored; full-result import-equals table covers root-export/recovered-exported forms, renamed/unrelated controls. Supplied E5-09/10/14/15 kills. R2-W3 is a different alias identity, expressly examined under question 4. |
| sol W3 | Closed for original indirect witnesses | Ordinary indirect and renamed JSX indirect reruns restore their full target, with the physical unresolved property placeholder still separate. Source-origin test and supplied E5-13 kill. The broader Source boundary is refuted separately by R2-W1/W2. |
| sol S1 | Closed | Reviewer alias/module-target control still retains `free_single`; new `e5_arm_a_binding_and_jsx_controls_hold_negative_target_constant` asserts full target/span/grade/no drop. Supplied E5-06 kills the missing-unbound-conjunct mutant. |
| Opus F1 | Closed | Fresh `t04`, `t04b`, `t23`, `t24`, `t29` preserve main targets; import-equals/awaited-import full-result tables and supplied E5-09/10/11/14/15 kills. |
| Opus F2 | Closed | Fresh `t01`, `t02`, `t05`, `t08`, `t27`, `t28`, `t32`, `t33` preserve their script targets; negative twins and E5-12/16/17/18 as above. |
| Opus F3 | Closed for original indirect witnesses | Fresh `t18`/`t20` restore indirect targets; `t19` preserves its earlier import rung. Source-origin control and supplied E5-13 kill. Its recommendation that Source-super remain refused is refuted by R2-W2. |
| Opus F4 | Closed | All six original `w2m`/`w2d` fixtures still drop at E5; fresh `d12s` stays Exact. Six added `r2_d12_w2{m,d}_script_{js,ts,tsx}` tests require retained full Exact targets/argument edges before outgoing-reachability assertions. PD-29 now selects them; supplied receipt shows all six fail behaviorally under the mutant. |
| Opus F5 | Closed | Removed Rust-only graph-target guard. Its only caller is `rust_scope_graph_resolution`, invoked behind the Rust-language check at `resolution.rs:2344-2407`. Sole live E5 call remains in unqualified R5, after same-file R4; redundant qualifier/cross-file checks removed. |
| Opus F6 | Closed | Full-result arm-(a) alias/JSX controls, both target merge directions plus serialization, and origin/exemption tests added. Supplied E5-05/06/07/08/13 kills demonstrate discrimination. Earlier-rung controls are explicitly labelled. |
| Opus F7 | Closed | Packet gives per-root and distinct-source counts. Reviewer aggregation finds 11 SB contributors and confirms the duplicate total.js roots have identical 97 site/target/verdict rows: 36 W/61 I each. Current 170 W/354 I per-root becomes 134 W/293 I distinct-source. |
| Opus F8 | Closed | Test renamed `b5_nested_declarators_follow_e5_and_preserve_imports`; packet enumerates ten changed values per grammar against main, four arm (a), six arm (b). Imported complete-value controls remain; selectors are reconciled. |

**Question 2 — the fold's mechanics.** The three-conjunct unbound exception implements R2 accurately: it protects possible JS script targets only for `Unproven("unbound")`, leaving local-binding, module-target, and non-JS cases subject to refusal. The exception is conservative target evidence, not a claim of global scope. The original correct-global witnesses pass.

The R1 collector's token/field reads and comment-skipping are sound for its specified forms. Import-require locals, namespace terminal properties, direct awaited-import operands, destructured local names, and the narrow exported-require recovery use syntax rather than suffix text. Recovery requires a missing semicolon and one semantic string argument; extra expressions, real semicolons, nested parentheses, and assertions are not silently accepted. The remaining different-terminal policy is refuted by R2-W3; trivia repair itself is not refuted.

The shared binder change is right for grammar-valid `import_alias`: the grammar starts with the local identifier, then an identifier/nested-identifier entity. Without leading comments the selected node is unchanged; with them it now selects the actual local name. It neither changes the general import-statement branch nor supplies a positive callable. Strictly, “only a first-child comment can change it” is a clean-syntax claim: `.find(identifier)` could also skip a preceding non-identifier recovery child such as ERROR. I did not demonstrate a changed resolution from such a malformed alias, and do not assert universal recovery equivalence. The supplied collector-only diagnostic and complete trivia table substantiate why the binder repair is needed. Dead-guard removal is sound for the current Rust-only call chain.

**Question 3 — arm (b)'s Source boundary.** Yes: R2-W1 and R2-W2 are concrete static bindings that main retains and the new head drops. The five corpus JSX WRONG rows and 23 INADMISSIBLE `super` rows do not establish safety outside those inputs. Origin-only is therefore insufficient as a universal preservation boundary. The original indirect defects are repaired; no physical member-qualifier loss is needed for these new witnesses.

**Question 4 — stated limits.** Yes: the expressly refused different-terminal namespace-import alias has a correct static cost (R2-W3). The CommonJS top-level and nested-script limits were also probed: new head retains main's `free_single` in both, exactly as R2 states.

Parenthesized awaited imports and the identity `.then(x => x)` fixture retain a main fallback and lose it on the new head; the plain awaited-import control is restored by R1. I do not claim these limits merely preserve main. I also do not add another WRONG from them: round 1's accepted Exact-contract reading excluded `import()` runtime re-acquisition, and R1 supplies conservative exemption evidence without proving positive identity. No new static-binding authority is inferred from the wrapper runs. No new defect was established for the assertion/recovered-invalid exported-require limits.

**Question 5 — other new material problems.** None beyond the three bounded findings above. Fact-builder consolidation preserves the arm-(a) set construction, constructors/removal/merge/serialization remain consistent, cache 115 is pinned, and D12 is unchanged with its discriminating selectors repaired. No new lifecycle or non-JS finding is raised.

**Execution, supplied evidence, and limits.** This reviewer ran **172 admitted frozen-CLI invocations**: 117 reruns across 39 round-1 fixtures/controls, 33 across 11 boundary fixtures, 10 boundary telemetry checks, and 12 namespace-limit checks. Every output was parsed and inspected; all had fresh cache directories. Two Node synthetic identity probes and one TypeScript 5.9.3 static check were run. Expectations, falsifiers, separating alternatives, outputs, and results are in `sol-scratch/probe-log.md` and the named JSON artifacts. An initial `nav dump-call-sites --help` attempt used a nonexistent subcommand; it was inadmissible and supplied no behavioral evidence. It was corrected to supported navigation queries.

Reviewer aggregation of the 758 supplied target adjudications matches X **19/0/11/1**, T **201/0/0/2**, SB **170/0/0/354**. The supplied 581-root head-to-head receipt reports zero call/graph differences and byte identity; this was read, not rerun. Private **109/0/0/0** removal and preservation of four awaited-import UNDECIDED targets remain controller-supplied aggregates; no private directory was opened. Zero classified-CORRECT corpus loss does not cover the synthetic static shapes above. CORRECT ReturnContains/ReturnInput consumer removals remain disclosed separately from Exact-target losses.

No cargo tests, suite, mutations, release build, or Tier-A runs were executed by this reviewer. Supplied current suite evidence is **5311 run: 5310 passed, 1 inherited example failure, 1 skipped**. The complete example assertion values have a supplied same-environment exact-main control; both MCP timing tests pass in the current suite receipt. Doctests **2/2**, scoped authoritative mutants **19/19 killed and admissible**, matrix **178/178**, valid identical TS/JS quick controls, and controller tests **14/14** are supplied evidence, not reviewer execution. I read the mutants' actual behavioral assertion failures, including all six PD-29 failures, rather than accepting exit status alone. Source/binary binding was independently checked as stated above.

Not checked afresh: full-suite attribution controls, actual mutation execution, all 162 packet fixtures, corpus captures/oracle correctness, inventory/navigation/non-call parity recapture, warm/partial cache transitions, live MCP, private corpus, Rust/full human-triggered Tier-A, full unscoped mutation lane, or landing. The four excluded SB producers remain excluded (atropa-ide timeout, clean-css exit -9, natural timeout, three timeout); none receives zero-cost credit. No implementation completion or all-green-suite claim is made.

**Final verdict: FIX. Findings: R2-W1 WRONG/MATERIAL; R2-W2 WRONG/MATERIAL; R2-W3 WRONG/MATERIAL.** Preserve the artifact; these are a closed enumerable set of boundary repairs with explicit tradeoffs, not grounds for a restart or a silent extension of the two-round cap.
