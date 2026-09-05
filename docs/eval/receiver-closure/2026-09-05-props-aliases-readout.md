# Module-local props object aliases

Base2aff090, PR249 merged and fetched. Owner approved the next bounded slice.
Implementation and publication are pending commit on feat/local-props-object-aliases.

## Result and boundary

TS/TSX destructured receivers now accept a single module-local props object alias:

```ts
type Props = {client: Client};
function explicit({client}: Props) { client.m(); }
const contextual: (p: Props) => void = ({client}) => client.m();
type F = (p: Props) => void;
const composed: F = ({client}) => client.m();
```

Each shape lookup terminates in a direct object/function RHS, with no recursive
expansion. Props visibility is checked at the annotation reference; Client identity
stays anchored to the Props declaration; write timing stays at the implementation
parameter. Exported and forward aliases work. Same-spelled value bindings do not
change type identity. Explicit annotations remain terminal, including failure.

Reuse the prior declaration collision/import/ambient/generic/shadow gates and
required-property, pattern and write barriers. Imported/nested/qualified aliases,
chains/cycles, interfaces, wrapped/union/intersection/mapped/conditional types,
class-member type aliases, React.FC and hooks remain excluded. No new JavaScript
type-alias semantics, resolution rung or serialized field. CPG70/nav39 invalidate
earlier receiver evidence. Production change is the shared private shape selector.

## Verification and review

Same-environment RED on untouched merged2aff090:2pass/1fail; all26 initial TS/TSX
positive variants lack Exact owner identity. Existing direct-object controls pass,
separating missing shape selection from the alternative broken imported-class
lookup. Initial GREEN3/0. Round2 expands to30 positive variants,69 declaration
barrier combinations and25 receiver barriers; all12 inline/contextual/alias groups
pass. Positive checks require the imported owner rather than a same-name decoy,
TypedParam resolution and full/direct-subset parity; erased imports stay erased.

Persisted good↔bad transitions cover explicit/contextual/composed paths. Props-RHS
A↔B changes replace cached owners in full/incremental builds for TS and TSX.
Navigation-sidecar round trips preserve positives and exclusions; old CPG69/nav38
artifacts miss. Cache-transition and owner-replacement groups each pass2/0.

Three-round SELF-PASS cap: round1 RED/implementation; round2 expanded boundary/cache
checks; round3 source trace through parameter recovery, write guards, classification,
owner lookup, call-site resolution and cache versions. No additional in-scope WRONG
identified, no extension. Prism navigation's indexed seed was unavailable;
SymbolNotFound was not absence evidence, and current-source inspection supplied
the consumer trace. No independent-review claim.

Full default3827/0/1 and MCP4017/0/1. Existing ignored test:
`resolution_test::slice_elem_variant_reserved`. Fmt/diff checks pass; clippy
completed with warnings retained. Immediately rebuilt matrix104/104. Immediately
rebuilt quick exit2: baseline-invalid solely corpus SHA drift2aff09050895 versus
pinned20c8490591a3. Oracle quiescent, oracle/SUT errors0, stale adjudications4.
No green quick gate, full multicorpus or baseline rewrite.

Pins: target-c-method flip_candidate, supplementary Exact5/0/0 and28 extra default
candidates; module-deps-feature-gated missing literal pin, oracle-only empty and
one MCP Prism-only; load-repo-feature-gated missing literal pin, oracle-only
resolution_test.rs:5299/5368 and four MCP Prism-only; ambiguous-symbol ok.
Sampled U-free callee Exact1/1/0 and recall gaps remain reported. No same-environment
base quick rerun for attribution: these are not claimed as regressions or fixes
from this slice. Full site lists and sampled counts are in the archived reports.

## Measurement

Saved pre-change release binary versus candidate on the same five-file Excalidraw
archive SHA0642e72cfa2d9a71198200e52f37399384610ee3:2780 stable records,
Exact376 on both sides, zero changed records. The six tracked React.FC/useContext
spans remain unproven. Earlier Black/Excalidraw/JavaScript controls byte-identical.
No real-site recall gain is claimed for this slice.

Separate served fixture: base exact-only callers and all three callee queries empty;
candidate serves explicit/app.ts:3, contextual/app.ts:4 and composed/app.ts:6 to
client.ts:2:m as TypedParam Exact. Optional, written, generic-shadowed and invalid
explicit annotations remain excluded. No fallback, warnings or truncation.
`verify-props-aliases.mjs` asserts these results; compact JSON and source hashes
are committed beside this readout.

## Custody and reproduction

Evidence root `/private/tmp/prism-props-aliases-MhRneP` contains base-prism and
candidate-prism, paired JSONL, served fixture sources/JSON and all gate logs.
Fixed source root `/private/tmp/prism-indirect-default-VUbv13/excalidraw`.
Run measure-indirect-default.mjs on the paired excalidraw JSONL and fixed root,
then verify-props-aliases.mjs on the evidence directory.
Raw archive, including fixed source:
`/private/tmp/prism-props-aliases-MhRneP-evidence.tgz`, SHA256
`dff7b2fb500c6be80fbc92943907618df7482de81d48fbf0cc55b23ba68d2cff`.
This run's untracked quick reports and prism-2aff09050895.json snapshot were moved
into the evidence root and remain recoverable; no tracked baseline was moved.
Original .superpowers/ and eval/snapshots/prism-fb81481dafa7.json preserved.
PR249 merge-record reconciliation is included; memory not edited.
