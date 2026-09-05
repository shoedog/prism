# Source-backed contextual-prop foundation

Base f348779, after PR245/246 merged. Owner explicitly chose "Source-backed
foundation first" after inspecting the ambient React.FC provenance gap.
Implementation and verification complete; publication pending.

## Result and boundary

TS/TSX direct variable-initializer arrows/function expressions can recover an
unannotated destructured receiver from a direct function-type annotation:

```ts
const run: (props: {client: Client}) => void = ({client}) => client.m();
```

One required parameter on each side, ordinary required inline property, simple
class type, no generic binders or parse recovery. Existing annotation evidence
takes precedence; failed explicit annotation proof cannot retry through context.
Reuse TypedParam and inline-property checks, keeping type visibility anchored at
the annotation and write timing anchored at the implementation parameter.
CPG67/nav36 invalidate stale recovered evidence. No new serialized field or rung.

Negative/edge coverage includes comments, renaming, expression/async/named function
bodies, nested captures, shadow/write timing, optional/rest/default/multiple
parameters, duplicate/accessor/index properties, aliases/unions/generics, wrappers,
assertions, generators and parse recovery. No source change after the initial
green implementation; round2 expanded test and persistence coverage.

This does NOT implement React.FC, imported/ambient generic signature resolution,
constructor this.field or hook-return receivers. The local provider returns None
for generics, while repo loading skips node_modules; an installed dependency or
the React.FC spelling alone is not contextual authority. The spec records the
counterexample and the owner's explicit choice of the smaller foundation.

## Verification and review

Same-environment complete RED on untouched f348779:2pass/1fail; the positive has
no recovered type, while exclusions pass. Arrow member ownership was already
covered in merged work. Initial green3/0; expanded round2-complete3/0. Two
zero-site fixtures were inadmissible and repaired with enclosing functions,
not misclassified as production defects. CPG positive↔optional/write-negative
transitions, contextual A→B/B→A persisted owner replacement and nav-sidecar
round trips pass. Full/subset parity and erased type-only imports are asserted.

Full default3818/0/1; MCP4008/0/1. Clippy complete (warnings retained);
fmt/diff checks clean. Rebuilt matrix104/104. No baseline rewrite/full multicorpus.
The existing ignored test is resolution_test::slice_elem_variant_reserved.

Immediately rebuilt quick exit2: baseline-invalid solely corpus SHA drift
f348779e16f3 vs pinned20c8490591a3; oracle quiescent, oracle/SUT errors0, stale
adjudications4. Pins: target-c-method flip_candidate (Exact5/0/0; 28 extra default
candidates); module-deps-feature-gated missing literal pin (oracle-only empty,
one MCP Prism-only); load-repo-feature-gated missing literal pin (oracle-only
resolution_test.rs:5299/5368, four MCP Prism-only); ambiguous-symbol ok.
Sampled U-free callee Exact1/1/0 and recall gaps remain reported, with no
same-environment base quick rerun for attribution. This is not a green accuracy
gate or evidence that those sampled errors were introduced/fixed here.

Three-round cap: round1 authority decision and RED; round2 expanded admissible
boundary/cache tests; round3 source/consumer/persistence trace found no further
in-scope defect. Self-pass, not independent review or whole-program type soundness.

## Measurement

Saved merged-base release vs candidate on the unchanged five-file Excalidraw
archive SHA0642e72cfa2d9a71198200e52f37399384610ee3:2780 site identities,
372 Exact on both sides, zero changed records. Four React.FC targets remain
unproven, as do the constructor-field/hook-return sites. Earlier Black/Excalidraw/
JavaScript controls are byte-identical. No new real-corpus recall claim.

Separate source-backed fixture: base exact-only callers/callees empty; candidate
serves visible→client.ts:2:m as TypedParam Exact, without fallback/warnings/
truncation. Optional and overwritten receivers do not become Exact callers.
This is an executable synthetic served-path gain, not a substituted real-site gain.

## Custody and reproduction

Evidence: /private/tmp/prism-contextual-props-wQOxSq, including saved base-prism,
candidate-prism, fixture sources, RED/gates, paired JSONL and served JSON.
Use measure-indirect-default.mjs on excalidraw-base/candidate.jsonl and the fixed
archive; verify-contextual-props.mjs asserts that result, served fixture and
byte-identical earlier controls. Its compact result is committed alongside this
readout. Original .superpowers/ and eval/snapshots/prism-fb81481dafa7.json untouched.
Raw evidence archive: /private/tmp/prism-contextual-props-wQOxSq-evidence.tgz,
SHA256 5e1519bab81c21fa4af99cdf5d22c89f45383f65a5f78adfd9912289c5f43073.
