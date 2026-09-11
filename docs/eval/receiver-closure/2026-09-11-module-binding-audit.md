# Module binding: executable baseline and refusal-first repair

Base: `afc78147` (merged PR #313). The predecessor's publication-held notes
describe its earlier local implementation checkpoint; #313 is merged.

## Scope and test meaning

`tests/integration/module_binding_audit_test.rs` defines 55 named syntax cases,
plus module-label, local-callable and serialization controls (58 tests).
Cases run across JS/TS/TSX and
full/subset-plus-export-resolution builds (type-only forms omit JS). Assertions
check Exact target file, declaration name and line, with same-spelled decoys.
They do not claim compiler type checking or unique-runtime-target authority.

Disposition is explicit: 13 Supported forms, 19 Gap forms, 22 Refused forms,
and one JS-supported form with a known TS/TSX parser gap. A passing Gap test
means **no Exact target is invented**, not that the feature is implemented.
The value-name `type` export case has two explicit parser-gap observations.

The separate four `module_binding_flow_test` cases inspect argument Use → exact
parameter-token Def endpoints in the CPG, including incremental rebuilds from
value-export/unshadowed states to type-only/shadowed states. These test consumer
effects rather than extrapolating flow correctness from helper output.

## Captured baseline and hypothesis log

All base measurements use unchanged production at afc78147 with identical new
tests in the retained detached worktree, on this host. No historical fixture
rows were invented. Logs live in `/private/tmp/prism-module-binding-LgUwcS`.

| Hypothesis / alternative | Discriminating observation | Classification |
|---|---|---|
| Comments shift module labels; alternative: module path cannot resolve | Plain require resolves; leading comments become the extracted module label; identical path succeeds after selector repair | WRONG: missing binding and parameter flow |
| Comments hide reflective writes; alternative: class identity is unavailable | Plain Object/Reflect mutation refuses; commented mutation retains Exact on base; candidate refuses both | WRONG: false Exact receiver |
| Ambient spellings are trusted; alternative: downstream name fallback | Shadowed/reassigned require/module/exports produces ImportMember to origin, not FreeSingle, on base | WRONG: false imported identity |
| exports is detached; alternative: duplicate property handling | `module.exports = {}; exports.publicName = origin` yields ImportMember on base despite no such exported member | WRONG: false exported identity |
| Type-only exports supply runtime targets; alternative: importer type-only filtering | Ordinary runtime import through statement/mixed-specifier type-only re-export reaches origin on base | WRONG: namespace crossing |
| Imported local export is treated as in-file declaration; alternative: real forwarding | A nested same-name declaration in bridge is selected instead of origin on base | WRONG: false local identity and parameter flow |
| Rejected local/import binding falls back globally; alternative: valid local callable | Base selects decoy FreeSingle for parameter, mutable require, whole/member require and type-only bindings; local callable controls remain Exact after repair | WRONG: false global identity |

Final baseline: 61 module/flow tests, **39 passed / 22 failed**. The 57-test
module matrix accounts for 18 failures; all four consumer-flow tests fail on
base. Reflective-write comment regression independently fails on base.
Initial candidate matrix: 57/57; consumer flows: 4/4; constructor-field suite:
5/5. The later serialization control independently fails on base and passes
on the candidate; its RED/GREEN logs are separate from the 61-test baseline.
The machine-readable baseline receipt preserves the matrix-run source hashes
and the later final test source hash rather than conflating those populations.

Across 316 matched synthetic fixture/language/build observations, six correct
Exact targets were recovered and 90 wrong Exact targets removed, affecting
17 named cases. These are not unique real call sites or corpus recall numbers.

Probe corrections retained: initial harness used nonexistent FunctionId.line
(compile-only, inadmissible); raw subset initially omitted its required
whole-program export-resolution phase (not a product regression). The value-name
`type` positive expectation exposed a genuine parser limitation: pinned
TypeScript 5.9.3 createSourceFile reports zero parse diagnostics for
`export { type as publicName } from './origin';`; Prism's TS/TSX grammar rejects
it. It is now an explicit parser-gap assertion, not a claimed resolution pass.

## Bounded implementation

- Reuse expression-argument filtering for both require extractors and reflective
  mutation targets, including comments within parentheses.
- Reuse lexical binding/write guards before recognizing CommonJS ambient names.
- Conservatively refuse `exports.*` when a syntactic whole-module replacement
  exists anywhere in the file. Reattachment/order reasoning is not supported.
- Keep statement-level and mixed-specifier type-only exports out of value facts.
- Poison imported-local Local export routes until actual forwarding provenance
  exists; same-name nested declarations cannot supply that missing proof.
- Persist syntactic module-value binding names as **refusal-only facts**. Before
  repo-global R5 fallback, reject a name bound by imports, type-only imports,
  module declarations or caller locals. Earlier local/import resolution is
  unchanged. This is not a complete lexical resolver and does not repair all
  same-file LocalDef ambiguities.
- CPG cache 86; navigation sidecar 47. No new forwarding or receiver capability.

The new module-name set intentionally does not change import eligibility:
reusing it there would reject the already-supported const destructured require
bindings. Coarse block aggregation and syntactic detachment can reduce recall;
neither is used to manufacture a new target.

## Verification status

Fresh matched release Tier-A matrix: **159/159**. Full Rust suites: default
**4,248**, MCP **4,441**, corrected MCP + detached-owner-audit **4,464 passed**;
zero failures and one existing `slice_elem_variant_reserved` ignore per run. Examples:
**32 passed**. Node: **786 unique tests passed** across the main run and the
explicit grammar-archive control rerun; callable-authority controls: **40**.
Three historical imported-props helper tests are excluded because their pinned
`real-sites.jsonl` input is unavailable. Python: **940 passed**, with only the
explicit live-adoption opt-in suite skipped.

Tier-A quick was terminated after 15 minutes 20 seconds without a completed
verdict. It is **incomplete, not green**; the generated snapshot is retained in
the local evidence directory. No quick recall/regression conclusion is claimed.

Independent read-only review completed round 2/2: **APPROVE, WRONG 0, SMELL 3**.
The smells are conservative nested/shadowed detachment, coarse block binding
aggregation, and the explicitly incomplete gates at review time. No source
review extension or post-review production edits.

First full-suite compile found one unit-test constructor missing the new refusal
field; corrected in that test fixture. The failed invocation is retained.
The initial owner-feature invocation lacked the explicit pinned compiler:
23 setup failures, not behavioral regression evidence; corrected run is separate.
Two initial full Python runs failed the 0.2-second warm gate. Cache metadata
proved the frozen CLI/MCP pair had different source-build identities (a test-only
source edit between builds also changes identity). Rebuilding the pair together
made the unchanged targeted control and full suite pass. Original artifacts and
failures remain in custody; one bounded full-run extension was disclosed after
repairing this verification-artifact mismatch. No timeout or production change.

## Remaining boundary

No alias/forwarding form was promoted. The 19 Gap cases are the next capability
baseline: namespace/member/whole-value imports and forwarding via ESM/CJS,
including same-name, renamed, object-export, direct-module and extra-local aliases.
Before expanding, prove the actual declaration across origin/local/export/consumer
links; name equality and missing competing targets are not proofs.

This finite fixture matrix is not an exhaustive audit of CommonJS heap effects,
computed exports, arbitrary assignment expressions, getters, proxy/alias effects,
or same-file fallback. Those require additional named cases and bounded proof
before claims of comprehensive module identity. Existing depth/cycle tests remain
in js_exports.rs and js_export_reexport_test.rs; no depth expansion occurred.
No React.FC/closure/package-resolution expansion or react-scripts change.
