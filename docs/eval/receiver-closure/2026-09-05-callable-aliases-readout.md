# Local single-call-signature object aliases

Base89f6ebf7, PR251 merged and fetched. Owner approved this bounded continuation.
Implementation/publication pending.

## Result and boundary

TS/TSX variable-initializer arrows/function expressions can recover destructured
receiver identity through a module-local callable-object alias:

```ts
type Plain = { (props: {client: Client}): void };
type Props = {client: Client};
type Named = { (props: Props): void };
type Generic<P> = { (props: P): void };
const a: Plain = ({client}) => client.m();
const b: Named = ({client}) => client.m();
const c: Generic<{client: Client}> = ({client}) => client.m();
const d: Generic<Props> = ({client}) => client.m();
```

The direct object RHS must contain exactly one call signature and no other
non-comment named member. Reject overloads even when one agrees, optional extra
properties, methods/accessors/index signatures, constructors, generic signatures,
interfaces, inheritance, wrapped/union/intersection/recursive shapes and imported/
ambient/qualified/nested aliases. Direct anonymous callable-object annotations stay
excluded. No React.FC/hook authority, inference or general TS assignability proof.

Normalization returns the original call_signature node. Existing unique alias,
generic binder/arity, required parameter/property, pattern, explicit annotation,
scope and write checks remain authoritative. Non-generic types use declaration
scope; generic concrete arguments keep use-site scope; implementation parameters
keep write timing. Comments/forward exports/trailing punctuation work; return
annotations do not authorize receiver recovery, including when omitted.

Production change is confined to contextual shape normalization. Props shape
lookup and owner resolution are unchanged. No new rung or serialized field;
CPG72/nav41 invalidate prior receiver evidence.

## Verification and review

Same-environment RED on untouched89f6ebf7:2pass/1fail; all28 initial TS/TSX positive
variants lack Exact owner edges. Existing generic function-alias positive control
passes1/0, separating the missing shape gate from the alternative broken owner path.
Initial GREEN3/0. Round2 expands to32 new positive variants plus2 explicit controls,
40 non-generic/generic shape negatives and40 authority/receiver negatives.
All18 inline/contextual/alias test groups pass. Positive assertions require the
imported owner rather than a same-name decoy, TypedParam, full/direct-subset parity,
and erased type-only runtime imports. Two obsolete single-call-object negative
rows migrated into positives; overload/extra-member exclusions remain explicit.

Persisted good↔overload, extra-member and generic-signature transitions pass.
Declaration/argument-only A↔B owner replacement covers all4 non-generic/generic ×
inline/Props paths in TS/TSX, with full/incremental call-site equality and exact
target-line assertions. Sidecar positives/exclusions and old CPG71/nav40 refusal
pass. Cache-transition and owner-replacement groups each pass2/0.

Three-round SELF-PASS cap: round1 RED/implementation, round2 boundary/cache
expansion, round3 source trace through normalization, parameter/write recovery,
classification, imported owner lookup and cached call sites. No further in-scope
WRONG identified, no extension or independent review. Prism-nav's indexed helper
was unavailable; SymbolNotFound was not absence evidence. Current source supplied
the consumer trace.

Default3833/0/1, MCP4023/0/1 and matrix104/104 verified. Clippy completed with
warnings retained. Immediately rebuilt quick exit2: baseline-invalid solely corpus
SHA drift89f6ebf7cd42 versus pinned20c8490591a3. Oracle quiescent, oracle/SUT errors0,
stale adjudications4. No green quick gate.
Existing ignored control: resolution_test::slice_elem_variant_reserved.
Fmt/diff checks pass; no full multicorpus or baseline rewrite.

Pins: target-c-method flip_candidate, supplementary Exact5/0/0 plus28 extra default
candidates; module-deps-feature-gated missing literal pin (oracle-only empty,
Prism-only src/mcp/tools.rs:230); load-repo-feature-gated missing literal pin
(oracle-only resolution_test.rs:5299/5368; Prism-only mcp/freshness.rs:401 and
mcp/session.rs:359/374/400); ambiguous-symbol ok.

Sampled Exact tp/fp/fn: callers C-method4/0/4, C-name36/0/0, Q-scoped0/0/1,
U-free2/0/0, U-method3/0/6; callees C-method4/0/2, C-name8/0/1, Q-scoped2/0/2,
U-free1/1/0, U-method10/0/0. No same-environment base quick rerun: these observations
are not attributed as regressions or fixes from this slice. Complete reports and
site lists retained, not rebaselined. An initial report-extraction command had a
syntax typo; that inadmissible probe was corrected before reading these results.

## Measurement

Saved base release versus candidate on the same five-file Excalidraw archive
SHA0642e72cfa2d9a71198200e52f37399384610ee3:2780 stable records, Exact376→376,
zero changed records. Six tracked React.FC/useContext spans still unproven; no
real-site recall gain claimed. Earlier Black/Excalidraw/JavaScript controls are
byte-identical.

Separate served fixture: base exact-only caller and4 callee queries empty;
candidate serves app.ts:4:plain, app.ts:6:named, app.ts:8:generic and app.ts:9:composed
to client.ts:2:m as TypedParam Exact. Overload, extra optional member, optional prop,
written receiver, invalid explicit annotation, argument shadow, anonymous callable
object and interface cases excluded. No fallback/warnings/truncation.
verify-callable-aliases.mjs asserts all results; compact JSON/hashes committed here.

## Custody and reproduction

Evidence /private/tmp/prism-callable-aliases-LhPrH3 contains saved binaries,
paired JSONL, fixture sources/served JSON and all logs. Fixed source root:
/private/tmp/prism-indirect-default-VUbv13/excalidraw. Run
measure-indirect-default.mjs on paired excalidraw JSONL plus fixed source, then
verify-callable-aliases.mjs on the evidence root. Raw archive including fixed source:
/private/tmp/prism-callable-aliases-LhPrH3-evidence.tgz, SHA256
`9a6ede7840d9490687a54a9ea5ed70afe23785802d42fa4c38c4c1ac58e76e64`.
Only this run's untracked quick reports and prism-89f6ebf7cd42.json snapshot moved
into recoverable evidence; no tracked baseline moved or rewritten.
Original .superpowers/ and eval/snapshots/prism-fb81481dafa7.json preserved.
PR251 merge records/successor handoff reconciled; memory not edited.
