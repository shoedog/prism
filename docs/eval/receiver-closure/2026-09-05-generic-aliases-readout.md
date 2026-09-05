# Bounded local generic contextual aliases

Base83594a8a, PR250 merged and fetched. Owner approved the proposed one-parameter
module-local generic function alias slice. Publication pending.

## Result and boundary

TS/TSX variable-initializer arrows/function expressions recover destructured
receivers through a direct local generic function alias:

```ts
type F<P> = (props: P) => void;
type Props = {client: Client};
const direct: F<{client: Client}> = ({client}) => client.m();
const aliased: F<Props> = ({client}) => client.m();
```

Exactly one plain unconstrained/non-defaulted binder, one explicit concrete
argument, and a direct non-generic function RHS with one required plain parameter.
The entire contextual parameter type must be the binder, not a nested member or
another alias. Comments, trailing commas, exports and forward references work.
Return types do not authorize receiver recovery; this is not full TS assignability
checking. No inference, constraints/defaults/variance, recursive substitution,
generic props aliases, imported/qualified/nested/ambient aliases, React.FC or hooks.

The original concrete argument syntax node reaches the object-shape checker.
F visibility stays at the name reference, Props at the argument, class identity
at the direct object/Props declaration, and writes at the implementation parameter.
Thus F<Client> may bind a parameter named Client without capturing the unrelated
class spelling inside a concrete object argument. Conversely, a nearer generic
Client around that argument blocks class identity. Explicit annotation failure
remains terminal. Required-property, duplicate/pattern and write barriers persist.

Private declaration selection is now separate from non-generic shape selection;
only the contextual path accepts the bounded generic form. No new resolution rung
or serialized field. CPG71/nav40 invalidate prior persisted receiver evidence.

## Verification and review

Same-environment RED on untouched83594a8a:2pass/1fail, all28 initial positive TS/TSX
variants fail Exact ownership. Existing local Props positive control passes1/0,
separating missing generic selection from the alternative broken class-owner path.
Initial GREEN3/0. Round2:32 generic positive variants plus2 explicit-annotation
controls,68 declaration-barrier combinations,39 argument/receiver negatives.
All15 inline/contextual/local-alias groups pass. Positive assertions require the
imported owner rather than a same-name decoy, TypedParam and full/direct-subset
parity, while preserving erased type-only runtime imports.

Persisted good↔bad transitions cover default-binder, optional-Props and write
changes. Argument-only A↔B changes replace owners for both direct objects and named
Props arguments in TS/TSX full and incremental builds. Sidecar positives/negatives
and old CPG70/nav39 refusal pass. Cache-transition and owner-replacement groups
each pass2/0.

Three-round SELF-PASS cap: round1 RED/implementation; round2 expanded scope/arity
and cache checks; round3 source trace through parameter recovery, type shadowing,
write guards, classification, imported owner lookup and persisted call sites.
No additional in-scope WRONG identified, no cap extension or independent review.
Prism-nav's indexed helper was unavailable; SymbolNotFound was not absence
evidence. Current-source inspection supplied the consumer trace.

Default3830/0/1, MCP4020/0/1 and matrix104/104 verified. Clippy completed with
warnings retained. Immediately rebuilt quick exit2: baseline-invalid solely SHA
drift83594a8aa3c3 versus pinned20c8490591a3. Oracle quiescent; oracle/SUT errors0;
stale adjudications4. No green quick gate.
Existing ignored test: resolution_test::slice_elem_variant_reserved.
Fmt/diff checks pass. No full multicorpus or rebaseline.

Pins: target-c-method flip_candidate, supplementary Exact5/0/0 plus28 extra default
candidates; module-deps-feature-gated missing literal pin (oracle-only empty,
Prism-only src/mcp/tools.rs:230); load-repo-feature-gated missing literal pin
(oracle-only resolution_test.rs:5299/5368; Prism-only mcp/freshness.rs:401 and
mcp/session.rs:359/374/400); ambiguous-symbol ok.

Sampled Exact tp/fp/fn: callers C-method4/0/4, C-name36/0/0, Q-scoped0/0/1,
U-free2/0/0, U-method3/0/6; callees C-method4/0/2, C-name8/0/1, Q-scoped2/0/2,
U-free1/1/0, U-method10/0/0. These accuracy gaps remain reported, not silently
rebaselined. No same-environment base quick rerun: neither sampled differences nor
pinned observations are attributed as regressions or fixes from this slice.

## Measurement

Saved base release versus candidate on the same five-file Excalidraw archive
SHA0642e72cfa2d9a71198200e52f37399384610ee3:2780 stable records, Exact376→376,
zero changed records. Six tracked React.FC/useContext-dependent spans remain
unproven; no real-site recall gain claimed. Earlier Black/Excalidraw/JavaScript
controls byte-identical.

Separate served fixture: base exact-only caller and all3 callee queries empty;
candidate serves app.ts:3:direct, app.ts:5:aliased and app.ts:7:captured to
client.ts:2:m as TypedParam Exact. Optional, written, argument-scope-shadowed,
invalid-explicit, constrained and extra-argument cases excluded. No fallback,
warnings or truncation. verify-generic-aliases.mjs asserts those results; compact
result and source hashes committed beside this readout.

## Custody and reproduction

Evidence root /private/tmp/prism-generic-aliases-sb86aA: saved base-prism and
candidate-prism, paired JSONL, fixture sources/served JSON, test/gate logs.
Fixed source: /private/tmp/prism-indirect-default-VUbv13/excalidraw.
Run measure-indirect-default.mjs on paired excalidraw JSONL plus fixed source,
then verify-generic-aliases.mjs on the evidence root. Raw archive including fixed
source: /private/tmp/prism-generic-aliases-sb86aA-evidence.tgz, SHA256
`f742ab74aab11e82a4586a3a4c84a3e627b6a2904a923f577341b6d21c75987d`.
Only this run's untracked quick JSON/Markdown and prism-83594a8aa3c3.json snapshot
were moved into recoverable evidence; no tracked baseline moved or rewritten.
Original .superpowers/ and eval/snapshots/prism-fb81481dafa7.json preserved.
PR250 merge records and successor handoff reconciled; memory not edited.
